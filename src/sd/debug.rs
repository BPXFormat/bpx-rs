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

use std::{collections::HashMap, convert::TryInto};

use crate::{
    error::Error,
    sd::{Array, Object},
    utils::hash,
    Result
};

/// Provides support for debug symbols to BPXSD object.
#[derive(Clone)]
pub struct DebugSymbols
{
    symbols_map: HashMap<u64, String>,
    symbols_list: Vec<String>
}

impl DebugSymbols
{
    /// Creates a new DebugSymbols.
    pub fn new() -> DebugSymbols
    {
        return DebugSymbols {
            symbols_list: Vec::new(),
            symbols_map: HashMap::new()
        };
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
    /// use bpx::sd::DebugSymbols;
    /// use bpx::utils::hash;
    ///
    /// let symbols = DebugSymbols::new();
    /// assert!(symbols.lookup(hash("Test")).is_none());
    /// ```
    pub fn lookup(&self, hash: u64) -> Option<&str>
    {
        if let Some(v) = self.symbols_map.get(&hash) {
            return Some(&v);
        }
        return None;
    }

    /// Pushes a new symbol in this symbol list.
    ///
    /// # Arguments
    ///
    /// * `symbol`:  the name of the symbol to push.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::DebugSymbols;
    /// use bpx::utils::hash;
    ///
    /// let mut symbols = DebugSymbols::new();
    /// symbols.push("Test");
    /// assert!(symbols.lookup(hash("Test")).is_some());
    /// ```
    pub fn push(&mut self, symbol: &str)
    {
        self.symbols_map.insert(hash(symbol), String::from(symbol));
        self.symbols_list.push(String::from(symbol));
    }

    /// Attach this symbol list to a BPXSD object.
    ///
    /// # Arguments
    ///
    /// * `obj`: the object to attach debug information to.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::{DebugSymbols, Object};
    /// use bpx::utils::hash;
    ///
    /// let mut symbols = DebugSymbols::new();
    /// symbols.push("Test");
    /// assert!(symbols.lookup(hash("Test")).is_some());
    /// let mut obj = Object::new();
    /// symbols.write(&mut obj);
    /// assert!(obj.get("__debug__").is_some());
    /// ```
    pub fn write(&self, obj: &mut Object)
    {
        obj.set("__debug__", self.symbols_list.clone().into());
    }

    /// Attempts to read debug information from a BPXSD object.
    ///
    /// # Arguments
    ///
    /// * `obj`: the object to read debug information from.
    ///
    /// returns: Result<DebugSymbols, Error>
    ///
    /// # Errors
    ///
    /// An [Error](crate::error::Error) is returned in case the object
    /// does not provide debug information or if the debug information
    /// could not be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::{DebugSymbols, Object};
    /// use bpx::utils::hash;
    ///
    /// let mut symbols = DebugSymbols::new();
    /// symbols.push("Test");
    /// let mut obj = Object::new();
    /// symbols.write(&mut obj);
    /// let symbols1 = DebugSymbols::read(&obj).unwrap();
    /// assert!(symbols1.lookup(hash("Test")).is_some());
    /// ```
    ///
    /// ```should_panic
    /// use bpx::sd::{DebugSymbols, Object};
    ///
    /// let mut obj = Object::new();
    /// DebugSymbols::read(&obj).unwrap();
    /// ```
    pub fn read(obj: &Object) -> Result<DebugSymbols>
    {
        if let Some(val) = obj.get("__debug__") {
            let mut symbols = HashMap::new();
            let val: &Array = val.try_into()?;
            for i in 0..val.len() {
                let sym: &str = (&val[i]).try_into()?;
                symbols.insert(hash(sym), String::from(sym));
            }
            return Ok(DebugSymbols {
                symbols_list: Vec::new(),
                symbols_map: symbols
            });
        }
        return Err(Error::MissingProp("__debug__"));
    }
}
