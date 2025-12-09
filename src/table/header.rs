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

//! Table header definition.

use bytesutil::{ReadBytes, WriteBytes};
use crate::core::header::Struct;
use crate::table::error::Error;

/// Size in bytes of a table header structure.
pub const SIZE_HEADER_STRUCTURE: usize = 8;

/// Table header structure.
pub struct Header {
    /// The address of this table name in the string section.
    pub name: u32,

    /// The number of columns in this table section.
    pub columns: u16
}

impl Struct<SIZE_HEADER_STRUCTURE> for Header {
    type Output = Header;
    type Error = Error;

    fn new() -> Self {
        Header {
            name: 0,
            columns: 0
        }
    }

    fn error_buffer_size() -> Option<Self::Error> {
        Some(Error::Eos)
    }

    fn from_bytes(buffer: [u8; SIZE_HEADER_STRUCTURE]) -> Result<Self::Output, Self::Error> {
        let name = u32::read_bytes_le(&buffer[0..4]);
        let columns = u16::read_bytes_le(&buffer[4..6]);
        Ok(Header { name, columns })
    }

    fn to_bytes(&self) -> [u8; SIZE_HEADER_STRUCTURE] {
        let mut buffer = [0; SIZE_HEADER_STRUCTURE];
        self.name.write_bytes_be(&mut buffer[0..4]);
        self.columns.write_bytes_be(&mut buffer[4..6]);
        buffer
    }
}
