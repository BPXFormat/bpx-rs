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

//! Error definition.

use std::fmt::{Display, Formatter};
use crate::core::error::OpenError;
use crate::impl_err_conversion;

/// The error type.
#[derive(Debug)]
pub enum Error {
    /// Indicates an invalid column type code.
    InvalidCode,

    /// Indicates the reader has reached EOS (End Of Section) before the end of the table header.
    Eos,

    /// Describes an io error.
    Io(std::io::Error),

    /// Indicates an [OpenError](OpenError) has occurred when attempting
    /// to open the section.
    Open(OpenError),

    /// A strings error.
    Strings(crate::strings::Error),

    /// Low-level BPX error.
    Bpx(crate::core::error::Error),

    /// Row index out of bounds.
    RowIndexOutOfBounds(usize),

    /// A column with the specified name could not be found.
    ColumnNotFound(String),
}

impl_err_conversion!(
    Error {
        std::io::Error => Io,
        OpenError => Open,
        crate::strings::Error => Strings,
        crate::core::error::Error => Bpx
    }
);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidCode => write!(f, "invalid data type code"),
            Error::Eos => write!(f, "got unexpected EOS"),
            Error::Io(e) => write!(f, "io error: {}", e),
            Error::Open(e) => write!(f, "section open error: {}", e),
            Error::Strings(e) => write!(f, "strings error: {}", e),
            Error::Bpx(e) => write!(f, "BPX error: {}", e),
            Error::RowIndexOutOfBounds(i) => write!(f, "row index out of bounds ({})", i),
            Error::ColumnNotFound(name) => write!(f, "column name not found ({})", name)
        }
    }
}
