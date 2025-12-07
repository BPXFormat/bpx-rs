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

use once_cell::unsync::OnceCell;

use crate::shader::{Target, Type};
use crate::{
    core::{
        header::{Struct, SECTION_TYPE_STRING},
        options::{Checksum, CompressionMethod, SectionOptions},
        Container, Handle,
    },
    shader::{
        decoder::{get_target_type_from_code, read_symbol_table},
        encoder::get_type_ext,
        error::{Error, Section},
        table::{ShaderTable, SymbolTable},
        Result, Settings, ShaderTableMut, ShaderTableRef, SymbolTableMut, SymbolTableRef,
        SECTION_TYPE_EXTENDED_DATA, SECTION_TYPE_SHADER, SECTION_TYPE_SYMBOL_TABLE,
        SUPPORTED_VERSION,
    },
    strings::StringSection,
};
use crate::util::table::NamedItemTable;
use super::{CreateOptions, OpenOptions, Options, DEFAULT_MAX_DEPTH};

/// A BPXS (ShaderPack).
///
/// # Examples
///
/// ```
/// use std::io::{Seek, SeekFrom};
/// use bpx::shader::{Shader, ShaderPack, Stage};
/// use bpx::shader::symbol;
/// use bpx::shader::symbol::FLAG_EXTENDED_DATA;
/// use bpx::util::new_byte_buf;
///
/// let mut bpxs = ShaderPack::create(new_byte_buf(0));
/// {
///     let mut symbols = bpxs.symbols_mut().unwrap();
///     symbols.create(symbol::Options::new("test")).unwrap();
/// }
/// {
///     let mut shaders = bpxs.shaders_mut();
///     shaders.create(Shader {
///         stage: Stage::Pixel,
///         data: Vec::new()
///     }).unwrap();
/// }
/// bpxs.save().unwrap();
/// //Reset the byte buffer pointer to start.
/// let mut bytebuf = bpxs.into_inner().into_inner();
/// bytebuf.seek(SeekFrom::Start(0)).unwrap();
/// //Attempt decoding the in-memory BPXS.
/// let mut bpxs = ShaderPack::open(bytebuf).unwrap();
/// let symbols = bpxs.symbols().unwrap();
/// let shaders = bpxs.shaders();
/// assert_eq!(symbols.len(), 1);
/// let last = symbols.iter().last().unwrap();
/// assert_eq!(symbols.load_name(last).unwrap(), "test");
/// assert_eq!(last.flags & FLAG_EXTENDED_DATA, 0);
/// let shader = shaders.load(shaders.iter().last().unwrap()).unwrap();
/// assert_eq!(shader.stage, Stage::Pixel);
/// assert_eq!(shader.data.len(), 0);
/// ```
pub struct ShaderPack<T> {
    settings: Settings,
    container: Container<T>,
    symbol_table: Handle,
    symbols: OnceCell<SymbolTable>,
    shaders: OnceCell<ShaderTable>,
    extended_data: Option<Handle>,
    max_depth: usize,
}

impl<T> ShaderPack<T> {
    /// Returns a reference to the settings of this package.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Returns a mutable reference to the settings of this package.
    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// Consumes this ShaderPack and returns the BPX container.
    pub fn into_inner(self) -> Container<T> {
        self.container
    }

    /// Returns a guard for mutable access to the symbol table.
    ///
    /// This returns None if the symbol table is not loaded. To load the symbol table, call
    /// the symbols() member function.
    pub fn symbols_mut(&mut self) -> Option<SymbolTableMut<'_, T>> {
        self.symbols.get_mut().map(|v| SymbolTableMut {
            table: v,
            container: &mut self.container,
        })
    }

    fn load_shader_table(&self) -> ShaderTable {
        let handles = self
            .container
            .sections()
            .iter()
            .filter(|v| self.container.sections()[*v].header().ty == SECTION_TYPE_SHADER)
            .collect();
        ShaderTable::new(handles)
    }

    /// Returns a guard for immutable access to the shader table.
    ///
    /// This will load the shader table if it's not already loaded.
    pub fn shaders(&self) -> ShaderTableRef<'_, T> {
        let table = self.shaders.get_or_init(|| self.load_shader_table());
        ShaderTableRef {
            container: &self.container,
            table,
        }
    }

    /// Returns a guard for mutable access to the shader table.
    ///
    /// This will load the shader table if it's not already loaded.
    pub fn shaders_mut(&mut self) -> ShaderTableMut<'_, T> {
        if self.shaders.get_mut().is_none() {
            //SAFETY: This is safe because only ran if the cell is none.
            unsafe {
                self.shaders
                    .set(self.load_shader_table())
                    .unwrap_unchecked()
            };
        }
        ShaderTableMut {
            container: &mut self.container,
            table: unsafe { self.shaders.get_mut().unwrap_unchecked() },
        }
    }
}

impl<T> TryFrom<Container<T>> for ShaderPack<T> {
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

impl<T> TryFrom<(Container<T>, Options)> for ShaderPack<T> {
    type Error = Error;

    fn try_from(
        (mut container, options): (Container<T>, Options),
    ) -> std::result::Result<Self, Self::Error> {
        match container.main_header().ty == b'S' {
            true => {
                if container.main_header().version != SUPPORTED_VERSION {
                    return Err(Error::BadVersion {
                        supported: SUPPORTED_VERSION,
                        actual: container.main_header().version,
                    });
                }
                let assembly_hash = container.main_header().type_ext.get_le(0);
                let (target, ty) = get_target_type_from_code(
                    container.main_header().type_ext[10],
                    container.main_header().type_ext[11],
                )?;
                let symbol_table =
                    match container.sections().find_by_type(SECTION_TYPE_SYMBOL_TABLE) {
                        Some(v) => v,
                        None => return Err(Error::MissingSection(Section::SymbolTable)),
                    };
                Ok(Self {
                    settings: Settings {
                        assembly_hash,
                        target,
                        ty,
                    },
                    symbol_table,
                    extended_data: container
                        .sections()
                        .find_by_type(SECTION_TYPE_EXTENDED_DATA),
                    container,
                    symbols: OnceCell::new(),
                    shaders: OnceCell::new(),
                    max_depth: options.max_depth,
                })
            }
            false => {
                container.main_header_mut().ty = b'S';
                container.main_header_mut().version = SUPPORTED_VERSION;
                let symbol_table = container.sections_mut().create(
                    SectionOptions::default()
                        .checksum(Checksum::Weak)
                        .compression(CompressionMethod::Zlib)
                        .ty(SECTION_TYPE_SYMBOL_TABLE),
                );
                let strings = StringSection::create(&mut container);
                let (target, ty) = get_target_type_from_code(
                    container.main_header().type_ext[10],
                    container.main_header().type_ext[11],
                )
                .unwrap_or((Target::Any, Type::Pipeline));
                Ok(Self {
                    container,
                    settings: Settings {
                        assembly_hash: 0,
                        target,
                        ty,
                    },
                    symbol_table,
                    symbols: OnceCell::from(SymbolTable::new(
                        NamedItemTable::empty(),
                        strings,
                        None,
                        options.max_depth,
                    )),
                    shaders: OnceCell::from(ShaderTable::new(Vec::new())),
                    extended_data: None,
                    max_depth: options.max_depth,
                })
            }
        }
    }
}

impl<T: Write + Seek> ShaderPack<T> {
    /// Creates a BPX type S.
    ///
    /// # Arguments
    ///
    /// * `backend`: A [Write](Write) + [Seek](Seek) to use as backend.
    /// * `settings`: The shader package creation settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::core::Container;
    /// use bpx::shader::ShaderPack;
    /// use bpx::util::new_byte_buf;
    ///
    /// let mut bpxs = ShaderPack::create(new_byte_buf(0));
    /// bpxs.save().unwrap();
    /// assert!(!bpxs.into_inner().into_inner().into_inner().is_empty());
    /// let mut bpxs = ShaderPack::try_from(Container::create(new_byte_buf(0))).unwrap();
    /// bpxs.save().unwrap();
    /// assert!(!bpxs.into_inner().into_inner().into_inner().is_empty());
    /// ```
    pub fn create(options: impl Into<CreateOptions<T>>) -> ShaderPack<T> {
        let options = options.into();
        let settings = options.settings;
        let mut container = Container::create(
            options
                .options
                .ty(b'S')
                .type_ext(get_type_ext(&settings))
                .version(SUPPORTED_VERSION),
        );
        let symbol_table = container.sections_mut().create(
            SectionOptions::default()
                .checksum(Checksum::Weak)
                .compression(CompressionMethod::Zlib)
                .ty(SECTION_TYPE_SYMBOL_TABLE),
        );
        let strings = StringSection::create(&mut container);
        Self {
            container,
            settings,
            symbol_table,
            symbols: OnceCell::from(SymbolTable::new(
                NamedItemTable::empty(),
                strings,
                None,
                options.max_depth,
            )),
            shaders: OnceCell::from(ShaderTable::new(Vec::new())),
            extended_data: None,
            max_depth: options.max_depth,
        }
    }

    /// Saves this shader package.
    ///
    /// This function calls **`save`** on the underlying BPX [Container](Container).
    ///
    /// # Errors
    ///
    /// Returns an [Error](Error) if some parts of this shader
    /// package couldn't be saved.
    pub fn save(&mut self) -> Result<()> {
        {
            //Update type ext if changed
            let data = get_type_ext(&self.settings);
            if data != self.container.main_header().type_ext {
                self.container.main_header_mut().type_ext = data;
            }
        }
        if let Some(syms) = self.symbols.get() {
            self.container
                .main_header_mut()
                .type_ext
                .set_le(8, syms.len() as u16);
            let mut section = self.container.sections().open(self.symbol_table)?;
            section.seek(SeekFrom::Start(0))?;
            section.clear();
            for v in syms {
                v.write(&mut *section)?;
            }
        }
        self.container.save()?;
        Ok(())
    }
}

impl<T: Read + Seek> ShaderPack<T> {
    /// Opens a BPX type S.
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
    /// use bpx::shader::ShaderPack;
    /// use bpx::util::new_byte_buf;
    ///
    /// let mut bpxs = ShaderPack::create(new_byte_buf(0));
    /// bpxs.save().unwrap();
    /// let mut buf = bpxs.into_inner().into_inner();
    /// buf.set_position(0);
    /// let mut bpxs = ShaderPack::open(buf).unwrap();
    /// let symbols = bpxs.symbols().unwrap();
    /// assert_eq!(symbols.len(), 0);
    /// ```
    pub fn open(options: impl Into<OpenOptions<T>>) -> Result<ShaderPack<T>> {
        let options = options.into();
        let container = Container::open(options.options)?;
        if container.main_header().ty != b'S' {
            return Err(Error::BadType {
                expected: b'S',
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

    fn load_symbol_table(&self) -> Result<SymbolTable> {
        let handle = self
            .container
            .sections()
            .find_by_type(SECTION_TYPE_STRING)
            .ok_or(Error::MissingSection(Section::Strings))?;
        let strings = StringSection::new(handle);
        let num_symbols = self.container.main_header().type_ext.get_le(8);
        let table = read_symbol_table(&self.container, num_symbols, self.symbol_table)?;
        Ok(SymbolTable::new(
            table,
            strings,
            self.extended_data,
            self.max_depth,
        ))
    }

    /// Returns a guard for immutable access to the symbol table.
    ///
    /// This will load the symbol table if it's not already loaded.
    ///
    /// # Errors
    ///
    /// An [Error](Error) is returned if the symbol table could not be
    /// loaded.
    pub fn symbols(&self) -> Result<SymbolTableRef<'_, T>> {
        let table = self.symbols.get_or_try_init(|| self.load_symbol_table())?;
        Ok(SymbolTableRef {
            table,
            container: &self.container,
        })
    }
}

impl<T: Read + Write + Seek> ShaderPack<T> {
    /// Saves this shader package.
    ///
    /// This function calls **`save`** on the underlying BPX [Container](Container).
    ///
    /// # Errors
    ///
    /// Returns an [Error](Error) if some parts of this shader
    /// package couldn't be saved.
    pub fn load_and_save(&mut self) -> Result<()> {
        {
            //Update type ext if changed
            let data = get_type_ext(&self.settings);
            if data != self.container.main_header().type_ext {
                self.container.main_header_mut().type_ext = data;
            }
        }
        if let Some(syms) = self.symbols.get() {
            self.container
                .main_header_mut()
                .type_ext
                .set_le(8, syms.len() as u16);
            let mut section = self.container.sections().load(self.symbol_table)?;
            section.seek(SeekFrom::Start(0))?;
            section.clear();
            for v in syms {
                v.write(&mut *section)?;
            }
        }
        self.container.load_and_save()?;
        Ok(())
    }
}
