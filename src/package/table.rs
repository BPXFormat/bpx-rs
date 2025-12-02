// Copyright (c) 2024, BlockProject 3D
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

use std::io::{Read, Seek, Write};
use std::ops::Index;

use crate::{
    core::{Container, Handle, SectionData},
    package::{
        decoder::unpack_object,
        encoder::{create_data_section_header, write_object, MAX_DATA_SECTION_SIZE},
        object::ObjectHeader,
        Result,
    },
    strings::{load_string_section, StringSection},
    table::NamedItemTable,
};

pub struct ObjectTable {
    strings: StringSection,
    table: NamedItemTable<ObjectHeader>,
    last_data_section: Option<Handle>,
}

impl ObjectTable {
    pub fn new(table: NamedItemTable<ObjectHeader>, strings: StringSection) -> ObjectTable {
        ObjectTable {
            strings,
            table,
            last_data_section: None,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ObjectHeader> {
        self.table.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn create<T, R: Read>(
        &mut self,
        container: &mut Container<T>,
        name: &str,
        mut source: R,
    ) -> Result<usize> {
        let mut object_size = 0;
        let mut data_section = *self.last_data_section.get_or_insert_with(|| {
            container
                .sections_mut()
                .create(create_data_section_header())
        });
        let start = container.sections()[data_section].index();
        let offset = {
            let section = container.sections().open(data_section)?;
            section.size()
        } as u32;

        loop {
            let (count, need_section) = write_object(container, &mut source, data_section)?;
            object_size += count;
            if need_section {
                data_section = container
                    .sections_mut()
                    .create(create_data_section_header());
            } else {
                break;
            }
        }
        let index;
        {
            // Fill and write the object header
            let buf = ObjectHeader {
                size: object_size as u64,
                name: self.strings.put(container, name)?,
                start,
                offset,
            };
            index = self.table.push(name.into(), buf);
        }
        {
            let section = container.sections().open(data_section)?;
            if section.size() > MAX_DATA_SECTION_SIZE {
                self.last_data_section = None;
            } else {
                self.last_data_section = Some(data_section);
            }
        }
        Ok(index)
    }

    pub fn new_data_section(&mut self) {
        self.last_data_section = None;
    }

    pub fn remove_at(&mut self, index: usize) {
        self.table.remove(index);
    }

    pub fn remove(&mut self, header: &ObjectHeader) {
        let (i, _) = self
            .table
            .iter()
            .enumerate()
            .find(|(_, v)| v == &header)
            .expect("attempt to remove a non-existent object header");
        self.remove_at(i);
    }

    pub fn load<T: Read + Seek, O: Write>(
        &self,
        container: &Container<T>,
        header: &ObjectHeader,
        out: O,
    ) -> Result<u64> {
        unpack_object(container, header, out)
    }

    pub fn load_name<T: Read + Seek>(
        &self,
        container: &Container<T>,
        header: &ObjectHeader,
    ) -> Result<&str> {
        load_string_section(container, &self.strings)?;
        let name = self.table.load_name(container, &self.strings, header)?;
        Ok(name)
    }

    pub fn find<T: Read + Seek>(
        &self,
        container: &Container<T>,
        name: &str,
    ) -> Result<Option<&ObjectHeader>> {
        load_string_section(container, &self.strings)?;
        let name = self.table.find_by_name(container, &self.strings, name)?;
        Ok(name)
    }
}

impl<'a> IntoIterator for &'a ObjectTable {
    type Item = &'a ObjectHeader;
    type IntoIter = std::slice::Iter<'a, ObjectHeader>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Index<usize> for ObjectTable {
    type Output = ObjectHeader;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

/// Immutable guard to the table of all objects in a BPXP.
pub struct ObjectTableRef<'a, T> {
    pub(crate) container: &'a Container<T>,
    pub(crate) table: &'a ObjectTable,
}

impl<T> ObjectTableRef<'_, T> {
    /// Gets all objects in this table.
    pub fn iter(&self) -> std::slice::Iter<'_, ObjectHeader> {
        self.table.iter()
    }

    /// Returns true if this table is empty.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of objects in this table.
    pub fn len(&self) -> usize {
        self.table.len()
    }
}

impl<'a, T> IntoIterator for &'a ObjectTableRef<'_, T> {
    type Item = &'a ObjectHeader;
    type IntoIter = std::slice::Iter<'a, ObjectHeader>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Index<usize> for ObjectTableRef<'_, T> {
    type Output = ObjectHeader;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

impl<T: Read + Seek> ObjectTableRef<'_, T> {
    /// Loads an object to the given `out` io backend.
    ///
    /// # Arguments
    ///
    /// * `out`: A [Write](Write) to unpack object data to.
    ///
    /// # Errors
    ///
    /// Returns an [Error](crate::package::error::Error) if the section couldn't be loaded
    /// or an IO error has occurred.
    pub fn load<O: Write>(&self, header: &ObjectHeader, out: O) -> Result<u64> {
        self.table.load(self.container, header, out)
    }

    /// Loads the name of an object if it's not already loaded.
    ///
    /// # Errors
    ///
    /// If the name is not already loaded, returns an [Error](crate::package::error::Error)
    /// if the section couldn't be loaded or the string couldn't be loaded.
    pub fn load_name(&self, header: &ObjectHeader) -> Result<&str> {
        self.table.load_name(self.container, header)
    }

    /// Lookup an object by its name.
    ///
    /// Returns None if the object does not exist.
    ///
    /// # Arguments
    ///
    /// * `name`: the name to search for.
    ///
    /// returns: Result<Option<&ObjectHeader>>
    ///
    /// # Errors
    ///
    /// An [Error](crate::package::error::Error) is returned if the strings could not be
    /// loaded.
    pub fn find(&self, name: &str) -> Result<Option<&ObjectHeader>> {
        self.table.find(self.container, name)
    }
}

/// Mutable guard to the table of all objects in a BPXP.
pub struct ObjectTableMut<'a, T> {
    pub(crate) container: &'a mut Container<T>,
    pub(crate) table: &'a mut ObjectTable,
}

impl<T> ObjectTableMut<'_, T> {
    /// Creates a new object in this package.
    ///
    /// Returns the index of the newly created object.
    ///
    /// # Arguments
    ///
    /// * `name`: The name of the object.
    /// * `source`: A [Read](Read) to read object data from.
    ///
    /// returns: Result<()>
    ///
    /// # Errors
    ///
    /// Returns an [Error](crate::package::error::Error) if the object couldn't be saved
    /// in this package.
    pub fn create<R: Read>(&mut self, name: &str, source: R) -> Result<usize> {
        self.table.create(self.container, name, source)
    }

    /// Removes an object from this package.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the object in the table to remove.
    ///
    /// # Panics
    ///
    /// This function may panic if the index is not in the table.
    pub fn remove_at(&mut self, index: usize) {
        self.table.remove_at(index);
    }

    /// Removes an object from this package.
    ///
    /// # Arguments
    ///
    /// * `header`: the object header identifying the object to remove.
    ///
    /// # Panics
    ///
    /// This function may panic if the given object header is not found in this package.
    pub fn remove(&mut self, header: &ObjectHeader) {
        self.table.remove(header);
    }

    /// Close the current data section and start recording a new one.
    ///
    /// This function forces objects created after calling this function to be recorded
    /// in a new data section.
    pub fn new_data_section(&mut self) {
        self.table.new_data_section();
    }
}
