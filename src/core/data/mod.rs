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

//! Utilities to manipulate the content of sections.

mod auto;
mod file;
mod memory;
mod util;

use std::io::{Read, Result, Seek, Write};

use crate::util::traits::{ReadToVec, Shift, ShiftTo};

/// Opaque variant intended to manipulate section data in the form of standard IO operations.
pub trait SectionData: Read + Write + Seek + ReadToVec {
    /// Truncates this section of `size` bytes. The new section size is returned.
    ///
    /// Once the section is truncated, bytes to be read after the truncation point are ignored.
    ///
    /// # Arguments
    ///
    /// * `size`: the number of bytes to chop from the end of the section.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::{Read, Seek, SeekFrom, Write};
    /// use bpx::core::{AutoSectionData, SectionData};
    ///
    /// let mut data = AutoSectionData::default();
    /// data.write_all(b"test").unwrap();
    /// data.truncate(2).unwrap();
    /// data.seek(SeekFrom::Start(0)).unwrap();
    /// let mut buf = [0; 4];
    /// data.read(&mut buf).unwrap();
    /// assert_eq!(std::str::from_utf8(&buf).unwrap(), "te\0\0");
    /// ```
    fn truncate(&mut self, size: usize) -> Result<usize>;

    /// Returns the current size of this section.
    fn size(&self) -> usize;
}

impl<T: SectionData> Shift for T {
    fn shift(&mut self, pos: ShiftTo) -> Result<()> {
        match pos {
            ShiftTo::Left(length) => util::shift_left(self, length as u32),
            ShiftTo::Right(length) => {
                let fuckingrust = self.size();
                util::shift_right(self, fuckingrust as u64, length as u32)
            }
        }
    }
}

pub use auto::AutoSectionData;

#[cfg(test)]
mod tests {
    use crate::core::AutoSectionData;
    use crate::util::traits::{Shift, ShiftTo};
    use std::io::{Read, Seek, SeekFrom, Write};

    const SEED: &str = "This is a test.";

    #[test]
    fn basic_shift_left() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        data.seek(SeekFrom::End(-4)).unwrap();
        data.shift(ShiftTo::Left(2)).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; SEED.len()];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "This is aest.t.");
    }

    #[test]
    fn basic_shift_right() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        data.seek(SeekFrom::End(-4)).unwrap();
        data.shift(ShiftTo::Right(2)).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; SEED.len() + 2];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "This is a tesest.");
    }

    #[test]
    fn shift_left_maintain_cursor() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        let cursor = data.seek(SeekFrom::End(-4)).unwrap();
        data.shift(ShiftTo::Left(2)).unwrap();
        assert_eq!(cursor, data.stream_position().unwrap());
    }

    #[test]
    fn shift_right_maintain_cursor() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        let cursor = data.seek(SeekFrom::End(-4)).unwrap();
        data.shift(ShiftTo::Right(2)).unwrap();
        assert_eq!(cursor, data.stream_position().unwrap());
    }

    #[test]
    fn long_shift_left() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        data.seek(SeekFrom::Start(4)).unwrap();
        data.shift(ShiftTo::Left(2)).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; SEED.len()];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "Th is a test.t.");
    }

    #[test]
    fn long_shift_right() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        data.seek(SeekFrom::Start(4)).unwrap();
        data.shift(ShiftTo::Right(2)).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; SEED.len() + 2];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "This i is a test.");
    }

    #[test]
    fn zero_shift_left() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        data.shift(ShiftTo::Left(2)).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; SEED.len()];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "This is a test.");
    }

    #[test]
    fn zero_shift_right() {
        let mut data = AutoSectionData::default();
        data.write_all(SEED.as_bytes()).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        data.shift(ShiftTo::Right(2)).unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; SEED.len() + 2];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "ThThis is a test.");
    }

    #[test]
    fn write_append() {
        let mut data = AutoSectionData::default();
        data.write_all("tt".as_bytes()).unwrap();
        data.seek(SeekFrom::Start(1)).unwrap();
        data.write_append(b"es").unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; 4];
        data.read(&mut buf).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), "test");
    }
}
