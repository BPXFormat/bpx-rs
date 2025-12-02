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

//! Provides support for debug symbols to BPXSD object.

use std::{borrow::Cow, collections::HashMap, convert::TryInto};

use crate::{
    hash::Name,
    sd::{error::TypeError, Array, Object, Value},
};

/// A BPXSD object debugger iterator.
pub struct Iter<'a> {
    inner: std::collections::hash_map::Iter<'a, Name, Value>,
    symbols_map: &'a HashMap<Name, String>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (Option<&'a str>, Name, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        let (mut k, mut v) = self.inner.next()?;
        while k == &"__debug__".into() {
            let (k1, v1) = self.inner.next()?;
            k = k1;
            v = v1;
        }
        Some((self.symbols_map.get(k).map(|v| &**v), *k, v))
    }
}

/// An owned debugger.
pub type ODebugger = Debugger<'static>;

/// A wrapper to BPXSD object with debugging capabilities.
#[derive(PartialEq, Clone)]
pub struct Debugger<'a> {
    inner: Cow<'a, Object>,
    symbols_map: HashMap<Name, String>,
    symbols_list: Vec<String>,
}

impl AsRef<Object> for Debugger<'_> {
    fn as_ref(&self) -> &Object {
        &self.inner
    }
}

impl<'a> From<&'a Object> for Cow<'a, Object> {
    fn from(v: &'a Object) -> Self {
        Self::Borrowed(v)
    }
}

impl From<Object> for Cow<'_, Object> {
    fn from(v: Object) -> Self {
        Self::Owned(v)
    }
}

impl<'a> Debugger<'a> {
    /// Attach a debugger to an object.
    ///
    /// # Arguments
    ///
    /// * `inner`: the object to attach the debugger to.
    ///
    /// returns: Result<Debugger, TypeError>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    /// use bpx::sd::debug::Debugger;
    ///
    /// let mut obj = Debugger::attach(Object::new()).unwrap();
    /// obj.set("Test", 12.into());
    /// let inner = obj.detach();
    /// assert_eq!(inner.len(), 2);
    /// assert!(inner.get("__debug__").is_some());
    /// assert!(inner.get("Test").is_some());
    /// ```
    pub fn attach<T: Into<Cow<'a, Object>>>(inner: T) -> Result<Debugger<'a>, TypeError> {
        let mut dbg = Debugger {
            inner: inner.into(),
            symbols_map: HashMap::new(),
            symbols_list: Vec::new(),
        };
        if let Some(val) = dbg.inner.get("__debug__") {
            let val: &Array = val.try_into()?;
            for i in 0..val.len() {
                let sym: &str = (&val[i]).try_into()?;
                dbg.symbols_map.insert(Name::from(sym), sym.into());
                dbg.symbols_list.push(sym.into());
            }
        }
        Ok(dbg)
    }

    /// Performs a lookup for a given hash value in this symbol list.
    /// Returns None if the symbol does not exist.
    ///
    /// # Arguments
    ///
    /// * `hash`: the hash for which to search the symbol name.
    ///
    /// returns: Option<&str>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::{Object, debug::Debugger};
    /// use bpx::hash::hash;
    ///
    /// let debugger = Debugger::attach(Object::new()).unwrap();
    /// assert!(debugger.lookup("Test").is_none());
    /// ```
    pub fn lookup<T: Into<Name>>(&self, name: T) -> Option<&str> {
        if let Some(v) = self.symbols_map.get(&name.into()) {
            return Some(v);
        }
        None
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
    /// use bpx::sd::debug::Debugger;
    ///
    /// let mut obj = Debugger::attach(Object::new()).unwrap();
    /// assert!(obj.as_ref().is_empty());
    /// obj.set("Test", 12.into());
    /// assert_eq!(obj.as_ref().len(), 1);
    /// ```
    pub fn set(&mut self, name: &str, value: Value) {
        let hash = Name::from(name);
        self.inner.to_mut().set(hash, value);
        self.symbols_list.push(name.into());
        self.symbols_map.insert(hash, name.into());
    }

    /// Iterate through the object keys, values and names.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.inner.as_ref().iter(),
            symbols_map: &self.symbols_map,
        }
    }

    /// Detaches the debugger from the inner object and return the inner object
    pub fn detach(mut self) -> Object {
        self.inner
            .to_mut()
            .set("__debug__", self.symbols_list.into());
        self.inner.into_owned()
    }
}

impl<'a> IntoIterator for &'a Debugger<'_> {
    type Item = (Option<&'a str>, Name, &'a Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
