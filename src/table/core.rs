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

//! The core BPX Table section implementation.

use crate::core::header::{Struct, SECTION_TYPE_TABLE};
use crate::core::options::{Checksum, CompressionMethod, SectionOptions};
use crate::core::{Container, Handle};
use crate::strings::{load_string_section, StringSection};
use crate::table::column::{Column, SIZE_COLUMN_STRUCTURE};
use crate::table::error::Error;
use crate::table::header::{Header, SIZE_HEADER_STRUCTURE};
use crate::table::row::{ColumnPos, Row};
use crate::table::table::{ColumnTable, ColumnTableMut, ColumnTableRef};
use crate::util::table::NamedItemTable;
use std::io::{Read, Seek, SeekFrom};

/// The core table type.
pub struct RawTable {
    handle: Handle,
    header: Header,
    col_table: ColumnTable,
}

impl RawTable {
    /// Opens a BPX [Table] section.
    ///
    /// # Arguments
    ///
    /// * `container`: the container in which the section is.
    /// * `handle`: a handle to the table section.
    /// * `strings`: a handle to the strings section to use.
    ///
    /// returns: Result<Table, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the table section header could not be loaded.
    pub fn open<T: Read + Seek>(
        container: &Container<T>,
        handle: Handle,
        strings: Handle,
    ) -> Result<Self, Error> {
        let (header, col_table) = load_column_table(container, handle, strings)?;
        Ok(Self {
            handle,
            header,
            col_table,
        })
    }

    /// Loads the name of this table from the string section.
    ///
    /// # Arguments
    ///
    /// * `container`: the container which owns the table section.
    ///
    /// returns: Result<&str, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the string section failed to load or if the name failed to load from
    /// the string section.
    pub fn load_name<T: Read + Seek>(&self, container: &Container<T>) -> Result<&str, Error> {
        load_string_section(container, &self.col_table.strings)?;
        Ok(self.col_table.strings.get(container, self.header.name)?)
    }

    fn set_name<T>(&mut self, container: &Container<T>, name: &str) -> Result<(), Error> {
        let addr = self.col_table.strings.put(container, name)?;
        self.header.name = addr;
        Ok(())
    }

    /// Creates a BPX [Table] section.
    ///
    /// # Arguments
    ///
    /// * `container`: the container which should own the table section.
    /// * `name`: the name of the table.
    /// * `strings`: the string section associated with this table section.
    ///
    /// returns: Result<Table, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the table name could not be written to the strings section.
    pub fn create<T>(
        container: &mut Container<T>,
        name: &str,
        strings: Handle,
    ) -> Result<Self, Error> {
        let handle = container.sections_mut().create(
            SectionOptions::default()
                .ty(SECTION_TYPE_TABLE)
                .compression(CompressionMethod::Xz)
                .checksum(Checksum::Crc32),
        );
        let mut tbl = Self::new(handle, strings);
        tbl.set_name(container, name)?;
        Ok(tbl)
    }

    fn new(handle: Handle, strings: Handle) -> Self {
        Self {
            handle,
            header: Header::new(),
            col_table: ColumnTable::new(NamedItemTable::empty(), StringSection::new(strings)),
        }
    }

    /// Saves the table header.
    ///
    /// # Arguments
    ///
    /// * `container`: the container which owns the table section.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the header could not be written to the section.
    pub fn save<T>(&mut self, container: &Container<T>) -> Result<(), Error> {
        let mut tbl_data = container.sections().open(self.handle)?;
        tbl_data.seek(SeekFrom::Start(0))?;
        self.header.columns = self.col_table.len() as _;
        self.header.write(&mut *tbl_data)?;
        for column in &self.col_table {
            column.write(&mut *tbl_data)?;
        }
        Ok(())
    }

    /// Returns a mutable handle to the column header table.
    ///
    /// # Arguments
    ///
    /// * `container`: the container which owns the table section.
    ///
    /// returns: ColumnTableMut<T>
    pub fn columns_mut<'a, T>(&'a mut self, container: &'a Container<T>) -> ColumnTableMut<'a, T> {
        ColumnTableMut {
            container,
            table: &mut self.col_table,
        }
    }

    /// Returns an immutable handle to the column header table.
    ///
    /// # Arguments
    ///
    /// * `container`: the container which owns the table section.
    ///
    /// returns: ColumnTableMut<T>
    pub fn columns<'a, T>(&'a self, container: &'a Container<T>) -> ColumnTableRef<'a, T> {
        ColumnTableRef {
            container,
            table: &self.col_table,
        }
    }

    /// Returns the size in bytes of a single row.
    pub fn get_row_size(&self) -> usize {
        let mut size = 0;
        for column in &self.col_table {
            size += column.get_size()
        }
        size
    }

    /// Returns the actual size in bytes of a row.
    pub fn get_actual_row_size(&self) -> usize {
        let size = self.get_row_size();
        if size.is_power_of_two() {
            (size + 1).next_power_of_two()
        } else {
            size.next_power_of_two()
        }
    }

    /// Returns the section handle.
    pub fn handle(&self) -> Handle {
        self.handle
    }

    /// Returns the [ColumnPos] structure associated with the given column from its name.
    ///
    /// # Arguments
    ///
    /// * `container`: the container which owns the table section.
    /// * `name`: the name of the column to find.
    ///
    /// returns: Result<ColumnPos, Error>
    ///
    /// # Errors
    ///
    /// An [Error] is returned if the string section could not be loaded, if a column name failed
    /// to load from the string section or if no column exists for the given name.
    pub fn get_column_pos<T: Read + Seek>(
        &self,
        container: &Container<T>,
        name: &str,
    ) -> Result<ColumnPos, Error> {
        let column = self.col_table.find(container, name)?;
        match column {
            Some(value) => {
                let mut offset = 0;
                for column in &self.col_table {
                    if column == value {
                        break;
                    }
                    offset += column.get_size();
                }
                Ok(ColumnPos {
                    offset,
                    len: value.get_size(),
                    ty: value.ty,
                })
            }
            None => Err(Error::ColumnNotFound(name.into())),
        }
    }

    /// Returns the [ColumnPos] structure associated with the given column from its index. Returns
    /// [None] when the index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the column to find.
    ///
    /// returns: Option<ColumnPos>
    pub fn get_column_pos_at(&self, index: usize) -> Option<ColumnPos> {
        let mut offset = 0;
        for (index1, column) in self.col_table.iter().enumerate() {
            if index1 == index {
                return Some(ColumnPos {
                    offset,
                    len: column.get_size(),
                    ty: column.ty,
                });
            }
            offset += column.get_size();
        }
        None
    }

    /// Creates a [Row] structure that fits this table definition.
    ///
    /// # Panics
    ///
    /// Panics if the size of a single row according to this table definition is 0
    /// (i.e. [get_row_size](Self::get_row_size) returned 0).
    pub fn alloc_row(&self) -> Row {
        assert!(self.get_row_size() > 0);
        Row::new(
            SIZE_HEADER_STRUCTURE + (self.col_table.len() * SIZE_COLUMN_STRUCTURE),
            self.get_row_size(),
            self.get_actual_row_size(),
        )
    }
}

fn load_column_table<T: Read + Seek>(
    container: &Container<T>,
    handle: Handle,
    strings: Handle,
) -> Result<(Header, ColumnTable), Error> {
    let mut tbl_data = container.sections().load(handle)?;
    let header = Header::read(&mut *tbl_data)?;
    let strings = StringSection::new(strings);
    let mut columns = Vec::new();
    for _ in 0..header.columns {
        let column = Column::read(&mut *tbl_data)?;
        columns.push(column);
    }
    Ok((
        header,
        ColumnTable::new(NamedItemTable::with_list(columns), strings),
    ))
}
