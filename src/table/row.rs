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

//! Row definition.

use std::io::SeekFrom;
use bytesutil::{ReadBytes, WriteBytes};
use crate::core::SectionData;
use crate::table::column::Type;
use crate::table::error::{Error, ValueError};

/// A pre-allocated row structure.
pub struct Row {
    data: Box<[u8]>,
    size: usize,
    header_size: usize
}

impl Row {
    pub(super) fn new(header_size: usize, size: usize, actual_size: usize) -> Row {
        Row {
            data: vec![0; actual_size].into_boxed_slice(),
            header_size,
            size
        }
    }

    /// Is this row marked as free.
    pub fn is_free(&self) -> bool {
        self.data[self.size] == 1
    }


    /// Sets the free flag for this row.
    ///
    /// # Arguments
    ///
    /// * `free`: true to mark this row as free, false otherwise.
    ///
    /// returns: ()
    pub fn set_free(&mut self, free: bool) {
        if free {
            self.data[self.size] = 1;
        } else {
            self.data[self.size] = 0;
        }
    }

    /// Returns the number of rows in the table.
    ///
    /// # Arguments
    ///
    /// * `section`: a reference to the low-level BPX [SectionData].
    ///
    /// returns: usize
    pub fn get_len<S: SectionData>(&self, section: &S) -> usize {
        let size = section.size() - self.header_size;
        let len = size / self.data.len();
        len
    }

    /// Reads the row at the given index into this structure.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the row in the section.
    /// * `section`: a reference to the low-level BPX [SectionData].
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the row data could not be read from the section.
    pub fn read<S: SectionData>(&mut self, index: usize, section: &mut S) -> crate::table::Result<()> {
        if index >= self.get_len(section) {
            return Err(Error::RowIndexOutOfBounds(index));
        }
        section.seek(SeekFrom::Start((self.header_size + index * self.data.len()) as _))?;
        section.read_exact(self.data.as_mut())?;
        Ok(())
    }

    /// Writes this row at the given index into the section.
    ///
    /// This overwrites any previous stored content of the row in the section.
    ///
    /// # Arguments
    ///
    /// * `index`: the index of the row in the section.
    /// * `section`: a reference to the low-level BPX [SectionData].
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the row data could not be written into the section.
    pub fn write<S: SectionData>(&self, index: usize, section: &mut S) -> crate::table::Result<()> {
        if index >= self.get_len(section) {
            return Err(Error::RowIndexOutOfBounds(index));
        }
        section.seek(SeekFrom::Start((self.header_size + index * self.data.len()) as _))?;
        section.write_all(&self.data)?;
        Ok(())
    }

    /// Writes this row at the end of the section.
    ///
    /// # Arguments
    ///
    /// * `section`: a reference to the low-level BPX [SectionData].
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the row data could not be written into the section.
    pub fn append<S: SectionData>(&self, section: &mut S) -> crate::table::Result<usize> {
        let index = self.get_len(section);
        section.seek(SeekFrom::End(0))?;
        section.write_all(&self.data)?;
        Ok(index)
    }

    /// Returns an immutable handle to the cell identified by the given [ColumnPos].
    pub fn cell(&self, pos: ColumnPos) -> CellRef<'_> {
        CellRef {
            data: &self.data[pos.offset..pos.offset + pos.len],
            ty: pos.ty
        }
    }

    /// Returns a mutable handle to the cell identified by the given [ColumnPos].
    pub fn cell_mut(&mut self, pos: ColumnPos) -> CellMut<'_> {
        CellMut {
            data: &mut self.data[pos.offset..pos.offset + pos.len],
            ty: pos.ty
        }
    }
}

/// The position of a column for accessing a given cell in a row.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ColumnPos {
    pub(super) offset: usize,
    pub(super) len: usize,
    pub(super) ty: Type
}

//FIXME: find a better API.

/// Represents the value stored in a cell.
pub trait Value<'a>: Sized {
    /// Reads the value from the cell.
    ///
    /// # Arguments
    ///
    /// * `cell`: the cell data buffer.
    /// * `ty`: the column data type.
    ///
    /// returns: Self
    fn read(cell: &'a [u8], ty: Type) -> Result<Self, ValueError>;

    /// Writes the value to the cell.
    ///
    /// # Arguments
    ///
    /// * `cell`: the cell data buffer.
    /// * `ty`: the column data type.
    ///
    /// returns: ()
    fn write(self, cell: &mut [u8], ty: Type) -> Result<(), ValueError>;
}

macro_rules! impl_integer_type {
    ($($t: ty)*) => {
        $(
            impl Value<'_> for $t {
                fn read(cell: &[u8], ty: Type) -> Result<Self, ValueError> {
                    match ty {
                        Type::Null => Err(ValueError::Null),
                        Type::Uint8 => Self::try_from(u8::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Uint16 => Self::try_from(u16::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Uint32 => Self::try_from(u32::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Uint64 => Self::try_from(u64::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int8 => Self::try_from(i8::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int16 => Self::try_from(i16::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int32 => Self::try_from(i32::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int64 => Self::try_from(i64::read_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        _ => Err(ValueError::IncompatibleType)
                    }
                }

                fn write(self, cell: &mut [u8], ty: Type) -> Result<(), ValueError> {
                    match ty {
                        Type::Null => Err(ValueError::Null),
                        Type::Uint8 => u8::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Uint16 => u16::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Uint32 => u32::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Uint64 => u64::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int8 => i8::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int16 => i16::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int32 => i32::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        Type::Int64 => i64::try_from(self).map(|v| v.write_bytes_le(cell)).map_err(|_| ValueError::Overflow),
                        _ => Err(ValueError::IncompatibleType)
                    }
                }
            }
        )*
    };
}

impl_integer_type!(u8 u16 u32 u64 i8 i16 i32 i64);

impl Value<'_> for bool {
    fn read(cell: &[u8], ty: Type) -> Result<Self, ValueError> {
        match ty {
            Type::Null => Err(ValueError::Null),
            Type::Boolean => Ok(bool::read_bytes_le(cell)),
            _ => Err(ValueError::IncompatibleType)
        }
    }

    fn write(self, cell: &mut [u8], ty: Type) -> Result<(), ValueError> {
        match ty {
            Type::Null => Err(ValueError::Null),
            Type::Boolean => {
                self.write_bytes_le(cell);
                Ok(())
            },
            _ => Err(ValueError::IncompatibleType)
        }
    }
}

impl Value<'_> for f64 {
    fn read(cell: &[u8], ty: Type) -> Result<Self, ValueError> {
        match ty {
            Type::Null => Err(ValueError::Null),
            Type::Double => Ok(f64::read_bytes_le(cell)),
            Type::Float => Ok(f32::read_bytes_le(cell) as _),
            _ => Err(ValueError::IncompatibleType)
        }
    }

    fn write(self, cell: &mut [u8], ty: Type) -> Result<(), ValueError> {
        match ty {
            Type::Null => Err(ValueError::Null),
            Type::Float => {
                (self as f32).write_bytes_le(cell);
                Ok(())
            },
            Type::Double => {
                self.write_bytes_le(cell);
                Ok(())
            },
            _ => Err(ValueError::IncompatibleType)
        }
    }
}

impl<'a> Value<'a> for &'a str {
    fn read(cell: &'a [u8], ty: Type) -> Result<Self, ValueError> {
        match ty {
            Type::Null => Err(ValueError::Null),
            Type::Varchar => std::str::from_utf8(cell).map(|v| v.trim_matches(&['\0'])).map_err(ValueError::Utf8),
            _ => Err(ValueError::IncompatibleType)
        }
    }

    fn write(self, cell: &mut [u8], ty: Type) -> Result<(), ValueError> {
        match ty {
            Type::Null => Err(ValueError::Null),
            Type::Varchar => {
                let bytes = self.as_bytes();
                if bytes.len() <= cell.len() {
                    cell[..bytes.len()].copy_from_slice(bytes);
                } else {
                    cell.copy_from_slice(&bytes[..cell.len()]);
                }
                Ok(())
            },
            _ => Err(ValueError::IncompatibleType)
        }
    }
}

/// A mutable cell handle.
pub struct CellMut<'a> {
    data: &'a mut [u8],
    ty: Type
}

impl CellMut<'_> {
    /// Returns the cell bytes as a mutable refence.
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data
    }

    /// Writes a value into this cell.
    ///
    /// # Arguments
    ///
    /// * `value`: the value to write.
    ///
    /// returns: ()
    pub fn set<'a, T: Value<'a>>(&mut self, value: T) -> Result<(), ValueError> {
        value.write(self.data, self.ty)
    }
}

/// An immutable cell handle.
pub struct CellRef<'a> {
    data: &'a [u8],
    ty: Type
}

impl<'a> CellRef<'a> {
    /// Returns the cell bytes as an immutable refence.
    pub fn as_bytes(&self) -> &[u8] {
        self.data
    }

    /// Reads a value from this cell.
    pub fn get<T: Value<'a>>(&self) -> Result<T, ValueError> {
        T::read(self.data, self.ty)
    }
}
