// Copyright (c) 2025, BlockProject 3D
//
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
//     * Redistributions of source code must retain the above copyright notice,
//       this list of conditions and the following disclaimer.
//     * Redistributions in binary form must reproduce the above copyright notice,
//       this list of conditions and the following disclaimer in the documentation
//       and/or other materials provided with the distribution.
//     * Neither the name of BlockProject 3D nor the names of its contributors
//       may be used to endorse or promote products derived from this software
//       without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::io::{Read, Seek, SeekFrom, Write};

use bytesutil::ByteBuf;
use once_cell::unsync::OnceCell;

use crate::package::{Architecture, Platform};
use crate::{
    core::{
        header::{Struct, SECTION_TYPE_SD, SECTION_TYPE_STRING},
        options::{Checksum, CompressionMethod, SectionOptions},
        Container, Handle,
    },
    package::{
        decoder::{get_arch_platform_from_code, read_object_table},
        encoder::get_type_ext,
        error::{Error, Section},
        table::{ObjectTable, ObjectTableMut, ObjectTableRef},
        Result, Settings, SECTION_TYPE_OBJECT_TABLE, SUPPORTED_VERSION,
    },
    sd::Value,
    strings::StringSection,
};
use crate::util::table::NamedItemTable;
use super::{CreateOptions, OpenOptions, Options, DEFAULT_MAX_DEPTH};

/// A BPXP (Package).
///
/// # Examples
///
/// ```
/// use std::io::{Seek, SeekFrom};
/// use bpx::util::new_byte_buf;
/// use bpx::package::Package;
/// use bpx::package::util::unpack_string;
///
/// let mut bpxp = Package::create(new_byte_buf(128)).unwrap();
/// {
///     let mut objects = bpxp.objects_mut().unwrap();
///     objects.create("TestObject", "This is a test 你好".as_bytes()).unwrap();
/// }
/// bpxp.save().unwrap();
/// //Reset the byte buffer pointer to start.
/// let mut bytebuf = bpxp.into_inner().into_inner();
/// bytebuf.seek(SeekFrom::Start(0)).unwrap();
/// //Attempt decoding the in-memory BPXP.
/// let mut bpxp = Package::open(bytebuf).unwrap();
/// let objects = bpxp.objects().unwrap();
/// assert_eq!(objects.len(), 1);
/// let last = objects.iter().last().unwrap();
/// assert_eq!(objects.load_name(last).unwrap(), "TestObject");
/// {
///     let s = unpack_string(&objects, last).unwrap();
///     assert_eq!(s, "This is a test 你好")
/// }
/// {
///     let wanted = objects.find("TestObject").unwrap().unwrap();
///     let s = unpack_string(&objects, wanted).unwrap();
///     assert_eq!(s, "This is a test 你好")
/// }
/// ```
pub struct Package<T> {
    settings: Settings,
    container: Container<T>,
    object_table: Handle,
    table: OnceCell<ObjectTable>,
    metadata: OnceCell<Value>,
    max_depth: usize,
}

impl<T> Package<T> {
    /// Returns a reference to the settings of this package.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Returns a mutable reference to the settings of this package.
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// Consumes this Package and returns the inner BPX container.
    pub fn into_inner(self) -> Container<T> {
        self.container
    }

    /// Returns a guard for mutable access to the object table.
    ///
    /// This returns None if the object table is not loaded. To load the object table, call
    /// the objects() member function.
    pub fn objects_mut(&mut self) -> Option<ObjectTableMut<'_, T>> {
        self.table.get_mut().map(|v| ObjectTableMut {
            table: v,
            container: &mut self.container,
        })
    }
}

impl<T> TryFrom<Container<T>> for Package<T> {
    type Error = Error;

    fn try_from(value: Container<T>) -> std::prelude::v1::Result<Self, Self::Error> {
        Self::try_from((
            value,
            Options {
                max_depth: DEFAULT_MAX_DEPTH,
            },
        ))
    }
}

impl<T> TryFrom<(Container<T>, Options)> for Package<T> {
    type Error = Error;

    fn try_from(
        (mut container, options): (Container<T>, Options),
    ) -> std::result::Result<Self, Self::Error> {
        match container.main_header().ty == b'P' {
            true => {
                if container.main_header().version != SUPPORTED_VERSION {
                    return Err(Error::BadVersion {
                        supported: SUPPORTED_VERSION,
                        actual: container.main_header().version,
                    });
                }
                let (a, p) = get_arch_platform_from_code(
                    container.main_header().type_ext[0],
                    container.main_header().type_ext[1],
                )?;
                let object_table =
                    match container.sections().find_by_type(SECTION_TYPE_OBJECT_TABLE) {
                        Some(v) => v,
                        None => return Err(Error::MissingSection(Section::ObjectTable)),
                    };
                Ok(Self {
                    settings: Settings {
                        metadata: None,
                        architecture: a,
                        platform: p,
                        type_code: [
                            container.main_header().type_ext[2],
                            container.main_header().type_ext[3],
                        ],
                    },
                    object_table,
                    container,
                    table: OnceCell::new(),
                    metadata: OnceCell::new(),
                    max_depth: options.max_depth,
                })
            }
            false => {
                container.main_header_mut().ty = b'P';
                container.main_header_mut().version = SUPPORTED_VERSION;
                let object_table = container.sections_mut().create(
                    SectionOptions::default()
                        .checksum(Checksum::Weak)
                        .compression(CompressionMethod::Zlib)
                        .ty(SECTION_TYPE_OBJECT_TABLE),
                );
                let strings = StringSection::create(&mut container);
                let (a, p) = get_arch_platform_from_code(
                    container.main_header().type_ext[0],
                    container.main_header().type_ext[1],
                )
                .unwrap_or((Architecture::Any, Platform::Any));
                Ok(Package {
                    metadata: OnceCell::new(),
                    settings: Settings {
                        metadata: None,
                        architecture: a,
                        platform: p,
                        type_code: [
                            container.main_header().type_ext[2],
                            container.main_header().type_ext[3],
                        ],
                    },
                    container,
                    object_table,
                    table: OnceCell::from(ObjectTable::new(NamedItemTable::empty(), strings)),
                    max_depth: options.max_depth,
                })
            }
        }
    }
}

impl<T: Write + Seek> Package<T> {
    /// Creates a new BPX type P.
    ///
    /// # Arguments
    ///
    /// * `backend`: A [Write](Write) + [Seek](Seek) to use as backend.
    /// * `settings`: The package creation settings.
    ///
    /// # Errors
    ///
    /// Returns an [Error](Error) if the metadata couldn't be created.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::core::Container;
    /// use bpx::package::Package;
    /// use bpx::util::new_byte_buf;
    ///
    /// let mut bpxp = Package::create(new_byte_buf(0)).unwrap();
    /// bpxp.save().unwrap();
    /// assert!(!bpxp.into_inner().into_inner().into_inner().is_empty());
    /// let mut bpxp = Package::try_from(Container::create(new_byte_buf(0))).unwrap();
    /// bpxp.save().unwrap();
    /// assert!(!bpxp.into_inner().into_inner().into_inner().is_empty());
    /// ```
    pub fn create(options: impl Into<CreateOptions<T>>) -> Result<Package<T>> {
        let options = options.into();
        let settings = options.settings;
        let mut container = Container::create(
            options
                .options
                .ty(b'P')
                .type_ext(get_type_ext(&settings))
                .version(SUPPORTED_VERSION),
        );
        let object_table = container.sections_mut().create(
            SectionOptions::default()
                .checksum(Checksum::Weak)
                .compression(CompressionMethod::Zlib)
                .ty(SECTION_TYPE_OBJECT_TABLE),
        );
        let strings = StringSection::create(&mut container);
        if let Some(metadata) = &settings.metadata {
            if !metadata.is_null() {
                let metadata_section = container.sections_mut().create(
                    SectionOptions::default()
                        .checksum(Checksum::Weak)
                        .compression(CompressionMethod::Zlib)
                        .ty(SECTION_TYPE_SD),
                );
                let mut section = container.sections().open(metadata_section)?;
                metadata.write(&mut *section, options.max_depth)?;
            }
        }
        Ok(Package {
            metadata: settings
                .metadata
                .clone()
                .map(OnceCell::from)
                .unwrap_or(OnceCell::new()),
            settings,
            container,
            object_table,
            table: OnceCell::from(ObjectTable::new(NamedItemTable::empty(), strings)),
            max_depth: options.max_depth,
        })
    }

    /// Saves this package.
    ///
    /// This function calls **`save`** on the underlying BPX [Container](Container).
    ///
    /// # Errors
    ///
    /// Returns an [Error](Error) if some parts of this package
    /// couldn't be saved.
    pub fn save(&mut self) -> Result<()> {
        //Update metadata section if changed
        if let Some(metadata) = &self.settings.metadata {
            if !metadata.is_null() {
                let handle = self
                    .container
                    .sections()
                    .find_by_type(SECTION_TYPE_SD)
                    .unwrap_or_else(|| {
                        self.container.sections_mut().create(
                            SectionOptions::default()
                                .checksum(Checksum::Weak)
                                .compression(CompressionMethod::Zlib)
                                .ty(SECTION_TYPE_SD),
                        )
                    });
                let mut section = self.container.sections().open(handle)?;
                metadata.write(&mut *section, self.max_depth)?;
            } else if let Some(handle) = self.container.sections().find_by_type(SECTION_TYPE_SD) {
                self.container.sections_mut().remove(handle);
            }
            self.metadata = OnceCell::from(metadata.clone());
        }
        {
            //Update type ext if changed
            let data = get_type_ext(&self.settings);
            if data != self.container.main_header().type_ext.as_ref() {
                self.container.main_header_mut().type_ext = ByteBuf::new(data);
            }
        }
        {
            let mut section = self.container.sections().open(self.object_table)?;
            section.seek(SeekFrom::Start(0))?;
            section.clear();
            if let Some(val) = self.table.get() {
                for v in val {
                    v.write(&mut *section)?;
                }
            }
        }
        self.container.save()?;
        Ok(())
    }
}

impl<T: Read + Seek> Package<T> {
    /// Opens a BPX type P.
    ///
    /// # Arguments
    ///
    /// * `backend`: A [Read](Read) + [Seek](Seek) to use as backend.
    ///
    /// # Errors
    ///
    /// An [Error](Error) is returned if some
    /// sections/headers could not be loaded.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::package::Package;
    /// use bpx::util::new_byte_buf;
    ///
    /// let mut bpxp = Package::create(new_byte_buf(0)).unwrap();
    /// bpxp.save().unwrap();
    /// let mut buf = bpxp.into_inner().into_inner();
    /// buf.set_position(0);
    /// let mut bpxp = Package::open(buf).unwrap();
    /// let objects = bpxp.objects().unwrap();
    /// assert_eq!(objects.len(), 0);
    /// ```
    pub fn open(options: impl Into<OpenOptions<T>>) -> Result<Package<T>> {
        let options = options.into();
        let container = Container::open(options.options)?;
        if container.main_header().ty != b'P' {
            return Err(Error::BadType {
                expected: b'P',
                actual: container.main_header().ty,
            });
        }
        Self::try_from((
            container,
            Options {
                max_depth: options.max_depth,
            },
        ))
    }

    fn load_object_table(&self) -> Result<ObjectTable> {
        let handle = self
            .container
            .sections()
            .find_by_type(SECTION_TYPE_STRING)
            .ok_or(Error::MissingSection(Section::Strings))?;
        let strings = StringSection::new(handle);
        let table = read_object_table(&self.container, self.object_table)?;
        Ok(ObjectTable::new(table, strings))
    }

    /// Returns a guard for immutable access to the object table.
    ///
    /// This will load the object table if it's not already loaded.
    ///
    /// # Errors
    ///
    /// An [Error](crate::shader::error::Error) is returned if the object table could not be
    /// loaded.
    pub fn objects(&self) -> Result<ObjectTableRef<'_, T>> {
        let table = self.table.get_or_try_init(|| self.load_object_table())?;
        Ok(ObjectTableRef {
            table,
            container: &self.container,
        })
    }

    /// Reads the metadata section of this BPXP if any.
    /// Returns None if there is no metadata in this BPXP.
    ///
    /// # Errors
    ///
    /// An [Error](Error) is returned in case of corruption or system error.
    pub fn load_metadata(&self) -> Result<&Value> {
        if self.metadata.get().is_none() {
            let res = match self.container.sections().find_by_type(SECTION_TYPE_SD) {
                Some(v) => {
                    let mut section = self.container.sections().load(v)?;
                    let obj = Value::read(&mut *section, self.max_depth)?;
                    self.metadata.set(obj)
                }
                None => self.metadata.set(Value::Null),
            };
            //SAFETY: This is safe because we're only running this if the cell is none.
            unsafe {
                res.unwrap_unchecked();
            }
        }
        //SAFETY: There's a check right before this line which inserts the value if it doesn't
        // exist.
        unsafe { Ok(self.metadata.get().unwrap_unchecked()) }
    }
}

impl<T: Read + Write + Seek> Package<T> {
    /// Saves this package.
    ///
    /// This function calls **`load_and_save`** on the underlying BPX [Container](Container).
    ///
    /// # Errors
    ///
    /// Returns an [Error](Error) if some parts of this package
    /// couldn't be saved.
    pub fn load_and_save(&mut self) -> Result<()> {
        //Update metadata section if changed
        if let Some(metadata) = &self.settings.metadata {
            if !metadata.is_null() {
                let handle = self
                    .container
                    .sections()
                    .find_by_type(SECTION_TYPE_SD)
                    .unwrap_or_else(|| {
                        self.container.sections_mut().create(
                            SectionOptions::default()
                                .checksum(Checksum::Weak)
                                .compression(CompressionMethod::Zlib)
                                .ty(SECTION_TYPE_SD),
                        )
                    });
                let mut section = self.container.sections().load(handle)?;
                metadata.write(&mut *section, self.max_depth)?;
            } else if let Some(handle) = self.container.sections().find_by_type(SECTION_TYPE_SD) {
                self.container.sections_mut().remove(handle);
            }
            self.metadata = OnceCell::from(metadata.clone());
        }
        {
            //Update type ext if changed
            let data = get_type_ext(&self.settings);
            if data != self.container.main_header().type_ext.as_ref() {
                self.container.main_header_mut().type_ext = ByteBuf::new(data);
            }
        }
        {
            let mut section = self.container.sections().load(self.object_table)?;
            section.seek(SeekFrom::Start(0))?;
            section.clear();
            if let Some(val) = self.table.get() {
                for v in val {
                    v.write(&mut *section)?;
                }
            }
        }
        self.container.load_and_save()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Seek, SeekFrom};

    use crate::package::util::unpack_string;
    use crate::package::{CreateOptions, OpenOptions};
    use crate::sd::Object;
    use crate::{package::Package, util::new_byte_buf};

    #[test]
    fn test_re_open_after_create() {
        let mut bpxp = Package::create(new_byte_buf(128)).unwrap();
        {
            let mut objects = bpxp.objects_mut().unwrap();
            objects
                .create("TestObject", "This is a test 你好".as_bytes())
                .unwrap();
        }
        bpxp.save().unwrap();
        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt decoding the in-memory BPXP.
        let mut bpxp = Package::open(bytebuf).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "TestObject");
        {
            let wanted = objects.find("TestObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "This is a test 你好")
        }
        //Attempt to write one more object into the file.
        bpxp.objects_mut()
            .unwrap()
            .create("AdditionalObject", "Another test".as_bytes())
            .unwrap();
        bpxp.save().unwrap();
        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt to re-decode the in-memory BPXP.
        let bpxp = Package::open(bytebuf).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "AdditionalObject");
        {
            let wanted = objects.find("TestObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "This is a test 你好")
        }
        {
            let wanted = objects.find("AdditionalObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "Another test")
        }
    }

    #[test]
    fn test_re_open_after_create_bad() {
        let mut bpxp = Package::create(new_byte_buf(128)).unwrap();
        {
            let mut objects = bpxp.objects_mut().unwrap();
            objects
                .create("TestObject", "This is a test 你好".as_bytes())
                .unwrap();
        }
        bpxp.save().unwrap();
        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt decoding the in-memory BPXP.
        let mut bpxp = Package::open(bytebuf).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "TestObject");
        //Do not load any other object to make sure the section is not loaded.
        //Attempt to write one more object into the file.
        bpxp.objects_mut()
            .unwrap()
            .create("AdditionalObject", "Another test".as_bytes())
            .unwrap();

        //Using save here is inapropriate because not all sections are loaded.
        assert!(bpxp.save().is_err());
        //Unfortunatly, due to the design of bpx-rs, at this point the underlying backend is corrupted because the save operation was interrupted.
    }

    #[test]
    fn test_re_open_after_create_bad_recover() {
        let mut bpxp = Package::create(new_byte_buf(128)).unwrap();
        {
            let mut objects = bpxp.objects_mut().unwrap();
            objects
                .create("TestObject", "This is a test 你好".as_bytes())
                .unwrap();
        }
        bpxp.save().unwrap();
        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt decoding the in-memory BPXP.
        let mut bpxp =
            Package::open(OpenOptions::new(bytebuf).revert_on_save_failure(true)).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "TestObject");
        //Do not load any other object to make sure the section is not loaded.
        //Attempt to write one more object into the file.
        bpxp.objects_mut()
            .unwrap()
            .create("AdditionalObject", "Another test".as_bytes())
            .unwrap();

        //Using save here is inapropriate because not all sections are loaded.
        assert!(bpxp.save().is_err());
        //In this case we use load_and_save to load all sections before saving.
        bpxp.load_and_save().unwrap();

        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt to re-decode the in-memory BPXP.
        let bpxp = Package::open(bytebuf).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "AdditionalObject");
        {
            let wanted = objects.find("TestObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "This is a test 你好")
        }
        {
            let wanted = objects.find("AdditionalObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "Another test")
        }
    }

    #[test]
    fn test_re_open_after_create_good() {
        let mut bpxp = Package::create(new_byte_buf(128)).unwrap();
        {
            let mut objects = bpxp.objects_mut().unwrap();
            objects
                .create("TestObject", "This is a test 你好".as_bytes())
                .unwrap();
        }
        bpxp.save().unwrap();
        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt decoding the in-memory BPXP.
        let mut bpxp = Package::open(bytebuf).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "TestObject");
        //Do not load any other object to make sure the section is not loaded.
        //Attempt to write one more object into the file.
        bpxp.objects_mut()
            .unwrap()
            .create("AdditionalObject", "Another test".as_bytes())
            .unwrap();

        //Using save here is inapropriate because not all sections are loaded.
        //In this case we use load_and_save to load all sections before saving.
        bpxp.load_and_save().unwrap();

        //Reset the byte buffer pointer to start.
        let mut bytebuf = bpxp.into_inner().into_inner();
        bytebuf.seek(SeekFrom::Start(0)).unwrap();
        //Attempt to re-decode the in-memory BPXP.
        let bpxp = Package::open(bytebuf).unwrap();
        let objects = bpxp.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let last = objects.iter().last().unwrap();
        assert_eq!(objects.load_name(last).unwrap(), "AdditionalObject");
        {
            let wanted = objects.find("TestObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "This is a test 你好")
        }
        {
            let wanted = objects.find("AdditionalObject").unwrap().unwrap();
            let s = unpack_string(&objects, wanted).unwrap();
            assert_eq!(s, "Another test")
        }
    }

    #[test]
    fn single_data_section() {
        let mut package = Package::create(new_byte_buf(4096)).unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject1", b"This is a new test".as_ref())
            .unwrap();
        package.save().unwrap();
        let container = package.into_inner();
        assert_eq!(container.sections().len(), 3);
        let mut buffer = container.into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();
        let package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a test");
        let s = unpack_string(&objects, &objects[1]).unwrap();
        assert_eq!(s, "This is a new test");
    }

    #[test]
    fn two_data_section() {
        let mut package = Package::create(new_byte_buf(4096)).unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package.objects_mut().unwrap().new_data_section();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject1", b"This is a new test".as_ref())
            .unwrap();
        package.save().unwrap();
        let container = package.into_inner();
        assert_eq!(container.sections().len(), 4);
        let mut buffer = container.into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();
        let package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a test");
        let s = unpack_string(&objects, &objects[1]).unwrap();
        assert_eq!(s, "This is a new test");
    }

    #[test]
    fn delete_object_1() {
        //Create the package with 2 objects
        let mut package = Package::create(new_byte_buf(4096)).unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject1", b"This is a new test".as_ref())
            .unwrap();
        package.save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Re-open the package and remove an object
        let mut package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let obj = *objects.find("TestObject").unwrap().unwrap();
        package.objects_mut().unwrap().remove(&obj);
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        package.load_and_save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Final integrity check
        let package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a new test");
    }

    #[test]
    fn delete_object_2() {
        //Create the package with 2 objects spread across 2 sections
        let mut package = Package::create(new_byte_buf(4096)).unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package.objects_mut().unwrap().new_data_section();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject1", b"This is a new test".as_ref())
            .unwrap();
        package.save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Re-open the package and remove an object
        let mut package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let obj = *objects.find("TestObject1").unwrap().unwrap();
        package.objects_mut().unwrap().remove(&obj);
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        package.load_and_save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Final integrity check
        let package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a test");
    }

    #[test]
    fn delete_object_2_recover() {
        //Create the package with 2 objects spread across 2 sections
        let mut package = Package::create(new_byte_buf(4096)).unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package.objects_mut().unwrap().new_data_section();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject1", b"This is a new test".as_ref())
            .unwrap();
        package.save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Re-open the package and remove an object
        let mut package =
            Package::open(OpenOptions::new(buffer).revert_on_save_failure(true)).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 2);
        let obj = *objects.find("TestObject").unwrap().unwrap();
        package.objects_mut().unwrap().remove(&obj);
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        package.load_and_save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Final integrity check
        let package = Package::open(OpenOptions::new(buffer).revert_on_save_failure(true)).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a new test");
    }

    #[test]
    fn add_remove_test() {
        //Create the package with 1 object
        let mut package = Package::create(new_byte_buf(4096)).unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package.save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Re-open the package and remove an object
        let mut package =
            Package::open(OpenOptions::new(buffer).revert_on_save_failure(true)).unwrap();
        package.objects().unwrap();
        package.objects_mut().unwrap().remove_at(0);
        package.load_and_save().unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a new test".as_ref())
            .unwrap();
        package.load_and_save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Final integrity check
        let package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 1);
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a new test");
    }

    #[test]
    fn add_remove_test_2() {
        //Create the package with 1 object
        let mut package =
            Package::create(CreateOptions::new(new_byte_buf(4096)).metadata(Object::new().into()))
                .unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject", b"This is a test".as_ref())
            .unwrap();
        package.save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Re-open the package and remove an object
        let mut package =
            Package::open(OpenOptions::new(buffer).revert_on_save_failure(true)).unwrap();
        package.load_metadata().unwrap();
        {
            let objects = package.objects().unwrap();
            let level = objects.find("BadName").unwrap();
            assert!(level.is_none());
        }
        package.objects().unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("BadName", b"This is a test".as_ref())
            .unwrap();
        package.load_and_save().unwrap();
        package
            .objects_mut()
            .unwrap()
            .create("TestObject1", b"This is a new test".as_ref())
            .unwrap();
        package.load_and_save().unwrap();
        package.objects_mut().unwrap().remove_at(1);
        package.load_and_save().unwrap();

        let mut buffer = package.into_inner().into_inner();
        buffer.seek(SeekFrom::Start(0)).unwrap();

        //Final integrity check
        let package = Package::open(buffer).unwrap();
        let objects = package.objects().unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects.load_name(&objects[0]).unwrap(), "TestObject");
        assert_eq!(objects.load_name(&objects[1]).unwrap(), "TestObject1");
        let s = unpack_string(&objects, &objects[0]).unwrap();
        assert_eq!(s, "This is a test");
        let s = unpack_string(&objects, &objects[1]).unwrap();
        assert_eq!(s, "This is a new test");
    }
}
