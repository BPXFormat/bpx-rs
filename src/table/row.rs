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
use std::ops::{Index, IndexMut};
use crate::core::SectionData;
use crate::table::error::Error;

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
}

impl Index<ColumnPos> for Row {
    type Output = [u8];

    fn index(&self, index: ColumnPos) -> &Self::Output {
        &self.data[index.offset..index.offset + index.len]
    }
}

impl IndexMut<ColumnPos> for Row {
    fn index_mut(&mut self, index: ColumnPos) -> &mut Self::Output {
        &mut self.data[index.offset..index.offset + index.len]
    }
}

/// The position of a column for accessing a given cell in a row.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ColumnPos {
    pub(super) offset: usize,
    pub(super) len: usize
}
