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

use bytesutil::{ReadBytes, WriteBytes};
use crate::core::header::Struct;
use crate::table::error::Error;
use crate::util::table::Item;

/// Size in bytes of a column structure.
pub const SIZE_COLUMN_STRUCTURE: usize = 8;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    Null,
    Boolean,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float,
    Double,
    Varchar
}

fn get_code_from_type(ty: Type) -> u8 {
    match ty {
        Type::Null => 0x0,
        Type::Boolean => 0x1,
        Type::Uint8 => 0x2,
        Type::Uint16 => 0x3,
        Type::Uint32 => 0x4,
        Type::Uint64 => 0x5,
        Type::Int8 => 0x6,
        Type::Int16 => 0x7,
        Type::Int32 => 0x8,
        Type::Int64 => 0x9,
        Type::Float => 0xA,
        Type::Double => 0xB,
        Type::Varchar => 0xC
    }
}

fn get_type_from_code(scode: u8) -> Result<Type, Error> {
    match scode {
        0x0 => Ok(Type::Null),
        0x1 => Ok(Type::Boolean),
        0x2 => Ok(Type::Uint8),
        0x3 => Ok(Type::Uint16),
        0x4 => Ok(Type::Uint32),
        0x5 => Ok(Type::Uint64),
        0x6 => Ok(Type::Int8),
        0x7 => Ok(Type::Int16),
        0x8 => Ok(Type::Int32),
        0x9 => Ok(Type::Int64),
        0xA => Ok(Type::Float),
        0xB => Ok(Type::Double),
        0xC => Ok(Type::Varchar),
        _ => Err(Error::InvalidCode),
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Column {
    pub name: u32,
    pub ty: Type,
    pub len: u16
}

impl Column {
    /// Returns the size in bytes of this column in a row.
    pub fn get_size(&self) -> usize {
        let init = match self.ty {
            Type::Null => 0,
            Type::Boolean => 1,
            Type::Uint8 => 1,
            Type::Uint16 => 2,
            Type::Uint32 => 4,
            Type::Uint64 => 8,
            Type::Int8 => 1,
            Type::Int16 => 2,
            Type::Int32 => 4,
            Type::Int64 => 8,
            Type::Float => 4,
            Type::Double => 8,
            Type::Varchar => 1
        };
        init * self.len as usize
    }
}

impl Struct<SIZE_COLUMN_STRUCTURE> for Column {
    type Output = Column;
    type Error = Error;

    fn new() -> Self {
        Column {
            name: 0,
            ty: Type::Null,
            len: 0
        }
    }

    fn error_buffer_size() -> Option<Self::Error> {
        Some(Error::Eos)
    }

    fn from_bytes(buffer: [u8; SIZE_COLUMN_STRUCTURE]) -> Result<Self::Output, Self::Error> {
        let name = u32::read_bytes_le(&buffer[0..4]);
        let ty = get_type_from_code(buffer[4])?;
        let len = u16::read_bytes_le(&buffer[5..7]);
        Ok(Column {
            name,
            ty,
            len
        })
    }

    fn to_bytes(&self) -> [u8; SIZE_COLUMN_STRUCTURE] {
        let mut data = [0; SIZE_COLUMN_STRUCTURE];
        self.name.write_bytes_le(&mut data[0..4]);
        data[4] = get_code_from_type(self.ty);
        self.len.write_bytes_le(&mut data[5..7]);
        data
    }
}

impl Item for Column {
    fn get_name_address(&self) -> u32 {
        self.name
    }
}
