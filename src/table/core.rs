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
use crate::core::{Container, Handle};
use crate::core::header::{Struct, SECTION_TYPE_TABLE};
use crate::core::options::{Checksum, CompressionMethod, SectionOptions};
use crate::strings::{load_string_section, StringSection};
use crate::table::column::Column;
use crate::table::error::Error;
use crate::table::header::Header;
use crate::table::table::{ColumnTable, ColumnTableMut, ColumnTableRef};
use crate::util::table::NamedItemTable;

pub struct Table {
    handle: Handle,
    header: Header,
    col_table: ColumnTable
}

impl Table {
    pub fn open<T: Read + Seek>(container: &Container<T>, handle: Handle, strings: Handle) -> Result<Self, Error> {
        let (header, col_table) = load_column_table(container, handle, strings)?;
        Ok(Self {
            handle,
            header,
            col_table
        })
    }

    pub fn load_name<T: Read + Seek>(&self, container: &Container<T>) -> Result<&str, Error> {
        load_string_section(container, &self.col_table.strings)?;
        Ok(self.col_table.strings.get(container, self.header.name)?)
    }

    fn set_name<T>(&mut self, container: &Container<T>, name: &str) -> Result<(), Error> {
        let addr = self.col_table.strings.put(container, name)?;
        self.header.name = addr;
        Ok(())
    }

    pub fn create<T>(container: &mut Container<T>, name: &str, strings: Handle) -> Result<Self, Error> {
        let handle = container.sections_mut().create(SectionOptions::default()
            .ty(SECTION_TYPE_TABLE)
            .compression(CompressionMethod::Xz)
            .checksum(Checksum::Crc32));
        let mut tbl = Self::new(handle, strings);
        tbl.set_name(container, name)?;
        Ok(tbl)
    }

    pub fn new(handle: Handle, strings: Handle) -> Self {
        Self {
            handle,
            header: Header::new(),
            col_table: ColumnTable::new(NamedItemTable::empty(), StringSection::new(strings))
        }
    }

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

    pub fn columns_mut<'a, T>(&'a mut self, container: &'a Container<T>) -> ColumnTableMut<'a, T> {
        ColumnTableMut {
            container,
            table: &mut self.col_table
        }
    }

    pub fn columns<'a, T>(&'a self, container: &'a Container<T>) -> ColumnTableRef<'a, T> {
        ColumnTableRef {
            container,
            table: &self.col_table
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
}

fn load_column_table<T: Read + Seek>(container: &Container<T>, handle: Handle, strings: Handle) -> Result<(Header, ColumnTable), Error> {
    let mut tbl_data = container.sections().load(handle)?;
    let header = Header::read(&mut *tbl_data)?;
    let strings = StringSection::new(strings);
    let mut columns = Vec::new();
    for _ in 0..header.columns {
        let column = Column::read(&mut *tbl_data)?;
        columns.push(column);
    }
    Ok((header, ColumnTable::new(NamedItemTable::with_list(columns), strings)))
}
