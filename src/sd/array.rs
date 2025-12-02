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

use std::vec::IntoIter;
use std::{
    borrow::{Borrow, BorrowMut},
    ops::{Deref, DerefMut, Index, IndexMut},
    slice::Iter,
    vec::Vec,
};

use crate::sd::Value;

/// Represents a BPX Structured Data Array.
#[derive(PartialEq, Clone)]
pub struct Array(Vec<Value>);

impl Default for Array {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Array {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Array {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Borrow<Vec<Value>> for Array {
    fn borrow(&self) -> &Vec<Value> {
        &self.0
    }
}

impl BorrowMut<Vec<Value>> for Array {
    fn borrow_mut(&mut self) -> &mut Vec<Value> {
        &mut self.0
    }
}

impl AsRef<Vec<Value>> for Array {
    fn as_ref(&self) -> &Vec<Value> {
        &self.0
    }
}

impl AsMut<Vec<Value>> for Array {
    fn as_mut(&mut self) -> &mut Vec<Value> {
        &mut self.0
    }
}

impl Array {
    /// Creates a new array.
    pub fn new() -> Array {
        Array(Vec::new())
    }

    /// Allocates a new array with a specified initial capacity
    pub fn with_capacity(capacity: u8) -> Array {
        Array(Vec::with_capacity(capacity as _))
    }

    /// Removes a value from the array.
    ///
    /// Returns None if the position is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `pos`: the position of the item in the array to remove.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Array;
    ///
    /// let mut arr = Array::new();
    /// arr.as_mut().push("Test".into());
    /// assert_eq!(arr.len(), 1);
    /// arr.remove(0);
    /// assert_eq!(arr.len(), 0);
    /// ```
    pub fn remove(&mut self, pos: usize) -> Option<Value> {
        if pos > self.0.len() {
            None
        } else {
            Some(self.0.remove(pos))
        }
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
    pub fn iter(&self) -> Iter<'_, Value> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a Array {
    type Item = &'a Value;
    type IntoIter = Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for Array {
    type Item = Value;
    type IntoIter = IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Index<usize> for Array {
    type Output = Value;

    fn index(&self, i: usize) -> &Value {
        &self.0[i]
    }
}

impl IndexMut<usize> for Array {
    fn index_mut(&mut self, i: usize) -> &mut Value {
        &mut self.0[i]
    }
}
