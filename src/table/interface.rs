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

use crate::core::{Container, Handle};
use crate::table::core::RawTable;
use crate::table::error::Error;
use crate::table::row::{ColumnPos, Row};
use crate::table::{ColumnTableMut, ColumnTableRef};
use std::io::{Read, Seek};

/// A high-level interface to [RawTable].
pub struct Table<'a, T> {
    table: RawTable,
    container: &'a Container<T>,
}

impl<'a, T> Table<'a, T> {
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
    pub fn create(
        container: &'a mut Container<T>,
        name: &str,
        strings: Handle,
    ) -> Result<Self, Error> {
        let table = RawTable::create(container, name, strings)?;
        Ok(Self { table, container })
    }

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
    pub fn open(container: &'a Container<T>, handle: Handle, strings: Handle) -> Result<Self, Error>
    where
        T: Read + Seek,
    {
        let table = RawTable::open(container, handle, strings)?;
        Ok(Self { table, container })
    }

    /// Loads the name of this table from the string section.
    ///
    /// returns: Result<&str, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the string section failed to load or if the name failed to load from
    /// the string section.
    pub fn load_name(&self) -> Result<&str, Error>
    where
        T: Read + Seek,
    {
        self.table.load_name(self.container)
    }

    /// Saves the table header.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the header could not be written to the section.
    pub fn save(&mut self) -> Result<(), Error> {
        self.table.save(self.container)
    }

    /// Returns a mutable handle to the column header table.
    pub fn columns_mut(&mut self) -> ColumnTableMut<'_, T> {
        self.table.columns_mut(self.container)
    }

    /// Returns an immutable handle to the column header table.
    pub fn columns(&self) -> ColumnTableRef<'_, T> {
        self.table.columns(self.container)
    }

    /// Returns the size in bytes of a single row.
    pub fn get_row_size(&self) -> usize {
        self.table.get_row_size()
    }

    /// Returns the actual size in bytes of a row.
    pub fn get_actual_row_size(&self) -> usize {
        self.table.get_actual_row_size()
    }

    /// Returns the section handle.
    pub fn handle(&self) -> Handle {
        self.table.handle()
    }

    /// Returns the [ColumnPos] structure associated with the given column from its name.
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the column to find.
    ///
    /// returns: Result<ColumnPos, Error>
    ///
    /// # Errors
    ///
    /// An [Error] is returned if the string section could not be loaded, if a column name failed
    /// to load from the string section or if no column exists for the given name.
    pub fn get_column_pos(&self, name: &str) -> Result<ColumnPos, Error>
    where
        T: Read + Seek,
    {
        self.table.get_column_pos(self.container, name)
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
        self.table.get_column_pos_at(index)
    }

    /// Creates a [Row] structure that fits this table definition.
    ///
    /// # Panics
    ///
    /// Panics if the size of a single row according to this table definition is 0
    /// (i.e. [get_row_size](Self::get_row_size) returned 0).
    pub fn alloc_row(&self) -> Row {
        self.table.alloc_row()
    }

    /// Returns the number of rows in the table.
    pub fn get_rows(&self) -> crate::table::Result<usize> {
        let data = self.container.sections().open(self.handle())?;
        Ok(crate::table::row::count(&*data, &self.alloc_row()))
    }

    /// Reads a row at the given index from this table.
    ///
    /// # Arguments
    ///
    /// * `row`: a pre-allocated row matching the table definition of `section`.
    /// * `index`: the index of the row in the section.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the row data could not be read from the section.
    pub fn read(&self, row: &mut Row, index: usize) -> crate::table::Result<()> {
        let mut data = self.container.sections().open(self.handle())?;
        crate::table::row::read(&mut *data, row, index)
    }

    /// Writes a row at the given index into the section.
    ///
    /// This overwrites any previous stored content of the row in the section.
    ///
    /// # Arguments
    ///
    /// * `row`: a pre-allocated row matching the table definition of `section`.
    /// * `index`: the index of the row in the section.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the row data could not be written into the section.
    pub fn write(&self, row: &Row, index: usize) -> crate::table::Result<()> {
        let mut data = self.container.sections().open(self.handle())?;
        crate::table::row::write(&mut *data, row, index)
    }

    /// Writes a row at the end of the section.
    ///
    /// # Arguments
    ///
    /// * `row`: a pre-allocated row matching the table definition of `section`.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the row data could not be written into the section.
    pub fn append(&self, row: &Row) -> crate::table::Result<usize> {
        let mut data = self.container.sections().open(self.handle())?;
        crate::table::row::append(&mut *data, row)
    }
}
