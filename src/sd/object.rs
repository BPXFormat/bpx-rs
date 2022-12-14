// Copyright (c) 2021, BlockProject 3D
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

use std::{
    collections::{hash_map::Keys, HashMap},
    ops::Index
};

use crate::{sd::Value, utils, Result};

/// Represents a BPX Structured Data Object.
#[derive(PartialEq, Clone)]
pub struct Object
{
    props: HashMap<u64, Value>
}

impl Object
{
    /// Creates a new object.
    pub fn new() -> Object
    {
        return Object { props: HashMap::new() };
    }

    /// Sets a property in the object using a raw property hash.
    ///
    /// # Arguments
    ///
    /// * `hash`: the BPX hash of the property.
    /// * `value`: the [Value](crate::sd::Value) to set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    ///
    /// let mut obj = Object::new();
    /// assert_eq!(obj.prop_count(), 0);
    /// obj.raw_set(0, 12.into());
    /// assert_eq!(obj.prop_count(), 1);
    /// ```
    pub fn raw_set(&mut self, hash: u64, value: Value)
    {
        self.props.insert(hash, value);
    }

    /// Sets a property in the object.
    ///
    /// # Arguments
    ///
    /// * `name`: the property name.
    /// * `value`: the [Value](crate::sd::Value) to set.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    ///
    /// let mut obj = Object::new();
    /// assert_eq!(obj.prop_count(), 0);
    /// obj.set("Test", 12.into());
    /// assert_eq!(obj.prop_count(), 1);
    /// ```
    pub fn set(&mut self, name: &str, value: Value)
    {
        self.raw_set(utils::hash(name), value);
    }

    /// Gets a property in the object by its hash.
    /// Returns None if the property hash does not exist.
    ///
    /// # Arguments
    ///
    /// * `hash`: the BPX hash of the property.
    ///
    /// returns: Option<&Value>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.raw_set(0, 12.into());
    /// assert!(obj.raw_get(0).is_some());
    /// assert!(obj.raw_get(1).is_none());
    /// ```
    pub fn raw_get(&self, hash: u64) -> Option<&Value>
    {
        return self.props.get(&hash);
    }

    /// Gets a property in the object.
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
    pub fn get(&self, name: &str) -> Option<&Value>
    {
        return self.raw_get(utils::hash(name));
    }

    /// Returns the number of properties in the object.
    pub fn prop_count(&self) -> usize
    {
        return self.props.len();
    }

    /// Returns a set of all key-value pairs (properties) in the object.
    pub fn get_keys(&self) -> Keys<'_, u64, Value>
    {
        return self.props.keys();
    }

    /// Attempts to write the object to the given IO backend.
    ///
    /// # Arguments
    ///
    /// * `dest`: the destination [Write](std::io::Write).
    ///
    /// returns: Result<(), Error>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    ///
    /// let mut obj = Object::new();
    /// obj.set("Test", 12.into());
    /// let mut buf = Vec::<u8>::new();
    /// obj.write(&mut buf);
    /// assert!(buf.len() > 0);
    /// ```
    pub fn write<TWrite: std::io::Write>(&self, dest: &mut TWrite) -> Result<()>
    {
        return super::encoder::write_structured_data(dest, self);
    }

    /// Attempts to read a BPXSD object from an IO backend.
    ///
    /// # Arguments
    ///
    /// * `source`: the source [Read](std::io::Read).
    ///
    /// returns: Result<Object, Error>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    /// use bpx::sd::Value;
    ///
    /// let mut obj = Object::new();
    /// obj.set("Test", 12.into());
    /// let mut buf = Vec::<u8>::new();
    /// obj.write(&mut buf);
    /// let obj1 = Object::read(&mut buf.as_slice()).unwrap();
    /// assert!(obj1.get("Test").is_some());
    /// assert!(obj1.get("Test").unwrap() == &Value::from(12));
    /// ```
    pub fn read<TRead: std::io::Read>(source: &mut TRead) -> Result<Object>
    {
        return super::decoder::read_structured_data(source);
    }
}

impl Index<&str> for Object
{
    type Output = Value;

    fn index(&self, name: &str) -> &Value
    {
        return &self.props.index(&utils::hash(name));
    }
}

impl Index<u64> for Object
{
    type Output = Value;

    fn index(&self, hash: u64) -> &Value
    {
        return &self.props.index(&hash);
    }
}
