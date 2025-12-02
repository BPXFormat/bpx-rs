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

//! Contains utilities to work with the object table section.

use crate::{
    core::header::Struct,
    package::error::{EosContext, Error},
};
use bytesutil::{ReadBytes, WriteBytes};
use crate::util::table::Item;

/// Size in bytes of an object header.
pub const SIZE_OBJECT_HEADER: usize = 20;

/// Represents an object header as read from the package.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ObjectHeader {
    /// The size of the object.
    pub size: u64,

    /// The pointer to the name of the object.
    pub name: u32,

    /// The start section index to the content.
    pub start: u32,

    /// The offset to the content in the start section.
    pub offset: u32,
}

impl Struct<SIZE_OBJECT_HEADER> for ObjectHeader {
    type Output = ObjectHeader;
    type Error = Error;

    fn new() -> Self {
        ObjectHeader {
            size: 0,
            name: 0,
            start: 0,
            offset: 0,
        }
    }

    fn error_buffer_size() -> Option<Self::Error> {
        Some(Error::Eos(EosContext::ObjectTable))
    }

    fn from_bytes(buffer: [u8; SIZE_OBJECT_HEADER]) -> Result<Self::Output, Self::Error> {
        let size = u64::read_bytes_le(&buffer[0..8]);
        let name_ptr = u32::read_bytes_le(&buffer[8..12]);
        let start = u32::read_bytes_le(&buffer[12..16]);
        let offset = u32::read_bytes_le(&buffer[16..20]);
        Ok(ObjectHeader {
            size,
            name: name_ptr,
            start,
            offset,
        })
    }

    fn to_bytes(&self) -> [u8; SIZE_OBJECT_HEADER] {
        let mut buf: [u8; SIZE_OBJECT_HEADER] = [0; SIZE_OBJECT_HEADER];
        self.size.write_bytes_le(&mut buf[0..8]);
        self.name.write_bytes_le(&mut buf[8..12]);
        self.start.write_bytes_le(&mut buf[12..16]);
        self.offset.write_bytes_le(&mut buf[16..20]);
        buf
    }
}

impl Item for ObjectHeader {
    fn get_name_address(&self) -> u32 {
        self.name
    }
}
