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

//! Contains an implementation of the BPXSD hashing function.

use std::{
    fmt::{Display, Formatter},
    num::Wrapping,
};

/// Convenient utility to wrap object property name hashes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Name(u64);

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Name {
    /// Returns the underlying hash code.
    pub fn into_inner(self) -> u64 {
        self.0
    }
}

impl From<u64> for Name {
    fn from(hash: u64) -> Self {
        Self(hash)
    }
}

impl<'a> From<&'a str> for Name {
    fn from(s: &'a str) -> Self {
        Self(hash(s))
    }
}

impl<'a> From<&'a String> for Name {
    fn from(s: &'a String) -> Self {
        Self(hash(s.as_ref()))
    }
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Self(hash(s.as_ref()))
    }
}

/// Hash text using the hash function defined in the BPX specification for strings.
///
/// # Arguments
///
/// * `s`: the string to compute the hash of.
///
/// returns: u64
///
/// # Examples
///
/// ```
/// use bpx::util::hash::hash;
///
/// let s = "MyString";
/// assert_eq!(hash(s), hash("MyString"));
/// assert_eq!(hash(s), hash(s));
/// assert_ne!(hash(s), hash("Wrong"));
/// ```
pub fn hash(s: &str) -> u64 {
    let mut val: Wrapping<u64> = Wrapping(5381);

    for v in s.as_bytes() {
        val = ((val << 5) + val) + Wrapping(*v as u64);
    }
    val.0
}
