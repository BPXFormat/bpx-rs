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

//! Column header table definition.

use crate::core::Container;
use crate::strings::{load_string_section, StringSection};
use crate::table::column::{Column, Type};
use crate::util::table::NamedItemTable;
use std::io::{Read, Seek};
use std::ops::Index;

pub struct ColumnTable {
    pub(super) strings: StringSection,
    table: NamedItemTable<Column>,
}

impl ColumnTable {
    pub fn new(table: NamedItemTable<Column>, strings: StringSection) -> ColumnTable {
        ColumnTable { strings, table }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Column> {
        self.table.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn create<T>(
        &mut self,
        container: &Container<T>,
        name: &str,
        ty: Type,
        len: u16,
    ) -> crate::table::Result<usize> {
        let address = self.strings.put(container, name)?;
        let buf = Column {
            name: address,
            ty,
            len,
        };
        Ok(self.table.push(name.into(), buf))
    }

    pub fn remove_at(&mut self, index: usize) {
        self.table.remove(index);
    }

    pub fn remove(&mut self, column: &Column) {
        let (i, _) = self
            .table
            .iter()
            .enumerate()
            .find(|(_, v)| v == &column)
            .expect("attempt to remove a non-existent object header");
        self.remove_at(i);
    }

    pub fn load_name<T: Read + Seek>(
        &self,
        container: &Container<T>,
        column: &Column,
    ) -> crate::table::Result<&str> {
        load_string_section(container, &self.strings)?;
        let name = self.table.load_name(container, &self.strings, column)?;
        Ok(name)
    }

    pub fn find<T: Read + Seek>(
        &self,
        container: &Container<T>,
        name: &str,
    ) -> crate::table::Result<Option<&Column>> {
        load_string_section(container, &self.strings)?;
        let name = self.table.find_by_name(container, &self.strings, name)?;
        Ok(name)
    }

    pub fn get(&self, index: usize) -> Option<&Column> {
        self.table.get(index)
    }
}

impl<'a> IntoIterator for &'a ColumnTable {
    type Item = &'a Column;
    type IntoIter = std::slice::Iter<'a, Column>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Index<usize> for ColumnTable {
    type Output = Column;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

/// Immutable guard to the table of all columns in a BPX Table Section.
pub struct ColumnTableRef<'a, T> {
    pub(crate) container: &'a Container<T>,
    pub(crate) table: &'a ColumnTable,
}

impl<T> ColumnTableRef<'_, T> {
    /// Gets all columns in this table.
    pub fn iter(&self) -> std::slice::Iter<'_, Column> {
        self.table.iter()
    }

    /// Returns true if this table is empty.
    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of columns in this table.
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// Gets immutable access to a column by its index.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the column.
    ///
    /// returns: Option<&Column>
    pub fn get(&self, index: usize) -> Option<&Column> {
        self.table.get(index)
    }
}

impl<'a, T> IntoIterator for &'a ColumnTableRef<'_, T> {
    type Item = &'a Column;
    type IntoIter = std::slice::Iter<'a, Column>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Index<usize> for ColumnTableRef<'_, T> {
    type Output = Column;

    fn index(&self, index: usize) -> &Self::Output {
        &self.table[index]
    }
}

impl<T: Read + Seek> ColumnTableRef<'_, T> {
    /// Loads the name of a column if it's not already loaded.
    ///
    /// # Errors
    ///
    /// If the name is not already loaded, returns an [Error](crate::table::error::Error)
    /// if the section couldn't be loaded or the string couldn't be loaded.
    pub fn load_name(&self, column: &Column) -> crate::table::Result<&str> {
        self.table.load_name(self.container, column)
    }

    /// Lookup a column by its name.
    ///
    /// Returns None if the column does not exist.
    ///
    /// # Arguments
    ///
    /// * `name`: the name to search for.
    ///
    /// returns: Result<Option<&Column>>
    ///
    /// # Errors
    ///
    /// An [Error](crate::package::error::Error) is returned if the strings could not be
    /// loaded.
    pub fn find(&self, name: &str) -> crate::table::Result<Option<&Column>> {
        self.table.find(self.container, name)
    }
}

/// Mutable guard to the table of all columns in a BPX Table Section.
pub struct ColumnTableMut<'a, T> {
    pub(crate) container: &'a Container<T>,
    pub(crate) table: &'a mut ColumnTable,
}

impl<T> ColumnTableMut<'_, T> {
    /// Creates a column into this BPX Table Section.
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the column.
    /// * `ty`: the data tyoe.
    /// * `len`: the data length.
    ///
    /// returns: Result<usize, Error>
    ///
    /// # Errors
    ///
    /// An [Error](Error) is returned if the column could not be
    /// written.
    pub fn create(&mut self, name: &str, ty: Type, len: u16) -> crate::table::Result<usize> {
        self.table.create(self.container, name, ty, len)
    }

    /// Removes a column from this table.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the column in the table to remove.
    ///
    /// # Panics
    ///
    /// This function may panic if the index is not in the table.
    pub fn remove_at(&mut self, index: usize) {
        self.table.remove_at(index)
    }

    /// Removes a column from this table.
    ///
    /// # Arguments
    ///
    /// * `column`: a reference to the column to remove.
    ///
    /// # Panics
    ///
    /// This function may panic if the given column is not found in this shader pack.
    pub fn remove(&mut self, column: &Column) {
        self.table.remove(column)
    }
}
