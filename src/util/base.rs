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

use std::{error::Error, fmt::Display, io::Cursor};

/// Creates a new in-memory byte buffer which can be used
/// to plug as IoBackend to a BPX encoder or decoder.
///
/// # Arguments
///
/// * `size`: the initial size of the buffer; if not known use 0.
///
/// returns: `Cursor<Vec<u8>>`
pub fn new_byte_buf(size: usize) -> Cursor<Vec<u8>> {
    if size > 0 {
        return Cursor::new(Vec::with_capacity(size));
    }
    Cursor::new(Vec::new())
}

/// An error which may be ignored by using an optional recovery option.
#[derive(Debug, PartialEq, Eq)]
pub struct RecoverableError<T, E: Error> {
    error: E,
    value: Option<T>,
}

impl<T, E: Error, E1: Error + Into<E>> From<E1> for RecoverableError<T, E> {
    fn from(value: E1) -> Self {
        RecoverableError {
            error: value.into(),
            value: None,
        }
    }
}

impl<T, E: Error> RecoverableError<T, E> {
    /// Create a new recoverable error with both an error and the recovery.
    pub fn new<E1: Into<E>>(error: E1, value: T) -> Self {
        RecoverableError {
            error: error.into(),
            value: Some(value),
        }
    }

    /// Returns the recovery value if any.
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Returns the underlying error.
    pub fn error(&self) -> &E {
        &self.error
    }

    /// Attempts to unwrap the recovery option.
    pub fn unwrap_value(self) -> T {
        match self.value {
            Some(v) => v,
            None => {
                panic!("attempt to unwrap the value of a recoverable error with no recovery option")
            }
        }
    }

    /// Returns the underlying error and drop the recovery option.
    pub fn into_error(self) -> E {
        self.error
    }

    /// Returns the underlying value if any and drop the error.
    pub fn into_value(self) -> Option<T> {
        self.value
    }
}

impl<T1, T, E: Error> From<RecoverableError<T, E>> for Result<T1, E> {
    fn from(value: RecoverableError<T, E>) -> Self {
        Err(value.into_error())
    }
}

impl<T: Display, E: Error> Display for RecoverableError<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(v) => write!(f, "recoverable error: {} ({})", self.error, v),
            None => write!(f, "{}", self.error),
        }
    }
}

/// Trait to allow unwrapping an error on either value or error.
pub trait UnwrapAny<T> {
    /// Unwraps a [Result] with an error type similar to the value type.
    fn unwrap_any(self) -> T;
}

impl<T> UnwrapAny<Option<T>> for Result<T, Option<T>> {
    fn unwrap_any(self) -> Option<T> {
        match self.map(Some) {
            Ok(v) => v,
            Err(v) => v,
        }
    }
}

impl<T> UnwrapAny<T> for Result<T, T> {
    fn unwrap_any(self) -> T {
        match self {
            Ok(v) => v,
            Err(v) => v,
        }
    }
}
