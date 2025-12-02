// Copyright (c) 2023, BlockProject 3D
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

//! This module provides a lookup-table style implementation.

use std::collections::HashMap;
use std::ops::Index;

use once_cell::unsync::OnceCell;

use crate::{core::Container, strings::StringSection};

/// Represents an item to be stored in an ItemTable.
pub trait Item {
    /// Returns the address of the name of this item in its string section.
    fn get_name_address(&self) -> u32;
}

/// Helper class to work with named items stored as arrays in a BPX container.
pub struct NamedItemTable<T> {
    list: Vec<T>,
    map: OnceCell<HashMap<String, usize>>,
}

impl<T> NamedItemTable<T> {
    /// Constructs a new NamedItemTable from a list of items.
    ///
    /// # Arguments
    ///
    /// * `list`: the list of items.
    pub fn with_list(list: Vec<T>) -> Self {
        Self {
            list,
            map: OnceCell::new(),
        }
    }

    /// Constructs a new empty NamedItemTable.
    pub fn empty() -> Self {
        Self {
            list: Vec::new(),
            map: OnceCell::from(HashMap::new()),
        }
    }

    /// Gets all items in this table.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.list.iter()
    }

    /// Returns true if this table is empty.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns the number of items in this table.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Gets immutable access to an item by its index.
    ///
    /// Returns None if the index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index`: the index to find.
    ///
    /// returns: Option<&T>
    pub fn get(&self, index: usize) -> Option<&T> {
        self.list.get(index)
    }

    /// Gets mutable access to an item by its index.
    ///
    /// Returns None if the index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `index`: the index to find.
    ///
    /// returns: Option<&mut T>
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.list.get_mut(index)
    }

    /// Removes an item from this table.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the item to remove.
    ///
    /// returns: ()
    pub fn remove(&mut self, index: usize) {
        self.list.remove(index);
        self.map = OnceCell::new();
    }
}

impl<T: Item> NamedItemTable<T> {
    /// Load the name of an item.
    ///
    /// # Arguments
    ///
    /// * `container`: the container to load the string from.
    /// * `strings`: the string section to use for loading the string.
    /// * `item`: the item to load the string of.
    ///
    /// returns: Result<&str, Error>
    ///
    /// # Errors
    ///
    /// An [Error](crate::strings::Error) is returned if the strings could not be loaded.
    pub fn load_name<'a, T1>(
        &self,
        container: &Container<T1>,
        strings: &'a StringSection,
        item: &T,
    ) -> Result<&'a str, crate::strings::Error> {
        strings.get(container, item.get_name_address())
    }

    /// Adds a new item to this table.
    ///
    /// # Arguments
    ///
    /// * `name`: the name of the item to add.
    /// * `item`: the item to add.
    ///
    /// returns: ()
    pub fn push(&mut self, name: String, item: T) -> usize {
        if let Some(map) = self.map.get_mut() {
            self.list.push(item);
            map.insert(name, self.list.len() - 1);
        } else {
            self.list.push(item);
        }
        self.len() - 1
    }

    fn build_lookup_table<T1>(
        &self,
        container: &Container<T1>,
        strings: &StringSection,
    ) -> Result<HashMap<String, usize>, crate::strings::Error> {
        let mut map: HashMap<String, usize> = HashMap::new();
        for (index, v) in self.list.iter().enumerate() {
            let name = strings.get(container, v.get_name_address())?.into();
            map.insert(name, index);
        }
        Ok(map)
    }

    /// Lookup an item by its name.
    /// Returns None if the item does not exist.
    ///
    /// # Arguments
    ///
    /// * `container`: the container to load strings from if needed.
    /// * `strings`: the string section to use for loading strings if needed.
    /// * `name`: the name to search for.
    ///
    /// returns: Result<Option<&T>, Error>
    ///
    /// # Errors
    ///
    /// A [Error](crate::strings::Error) is returned if the strings could not be loaded.
    pub fn find_by_name<T1>(
        &self,
        container: &Container<T1>,
        strings: &StringSection,
        name: &str,
    ) -> Result<Option<&T>, crate::strings::Error> {
        let map = self
            .map
            .get_or_try_init(|| self.build_lookup_table(container, strings))?;
        Ok(match map.get(name) {
            Some(index) => self.list.get(*index),
            None => None,
        })
    }
}

impl<'a, T> IntoIterator for &'a NamedItemTable<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> Index<usize> for NamedItemTable<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.list[index]
    }
}
