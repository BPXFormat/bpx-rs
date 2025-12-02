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

//! BPXSD object definition

use std::{
    collections::{hash_map::Iter, HashMap},
    ops::Index,
};

use crate::{hash::Name, sd::Value};

/// Represents a BPX Structured Data Object.
#[derive(PartialEq, Clone)]
pub struct Object(HashMap<Name, Value>);

impl AsRef<HashMap<Name, Value>> for Object {
    fn as_ref(&self) -> &HashMap<Name, Value> {
        &self.0
    }
}

impl Default for Object {
    fn default() -> Self {
        Self::new()
    }
}

impl Object {
    /// Creates a new object.
    pub fn new() -> Object {
        Object(HashMap::new())
    }

    /// Allocates a new object with a specified initial capacity
    pub fn with_capacity(capacity: u8) -> Object {
        Object(HashMap::with_capacity(capacity as _))
    }

    /// Sets a property in the object.
    ///
    /// # Arguments
    ///
    /// * `name`: the property name.
    /// * `value`: the [Value] to set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    ///
    /// let mut obj = Object::new();
    /// assert!(obj.is_empty());
    /// obj.set("Test", 12.into());
    /// assert_eq!(obj.len(), 1);
    /// ```
    pub fn set<T: Into<Name>>(&mut self, name: T, value: Value) {
        self.0.insert(name.into(), value);
    }

    /// Convenience function to quickly get a property by its name.
    ///
    /// Returns None if the property name does not exist.
    ///
    /// # Arguments
    ///
    /// * `name`: the property name.
    ///
    /// returns: Option<&Value>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    /// use bpx::sd::Value;
    ///
    /// let mut obj = Object::new();
    /// obj.set("Test", 12.into());
    /// assert!(obj.get("Test").is_some());
    /// assert!(obj.get("Test1").is_none());
    /// assert!(obj.get("Test").unwrap() == &Value::from(12));
    /// ```
    pub fn get<T: Into<Name>>(&self, name: T) -> Option<&Value> {
        self.0.get(&name.into())
    }

    /// Returns the number of properties in the object.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns whether this object is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate through the object keys, values and names.
    pub fn iter(&self) -> Iter<'_, Name, Value> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = (&'a Name, &'a Value);
    type IntoIter = Iter<'a, Name, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_ref().iter()
    }
}

impl<T: Into<Name>> Index<T> for Object {
    type Output = Value;

    fn index(&self, name: T) -> &Value {
        self.0.index(&name.into())
    }
}
