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

use std::io::{Cursor, Read, Result, Seek, SeekFrom, Write};

use crate::util::traits::ReadToVec;
use crate::{core::SectionData, util::new_byte_buf};

pub struct InMemorySection {
    byte_buf: Cursor<Vec<u8>>,
    cur_size: usize,
}

impl InMemorySection {
    pub fn new(initial: usize) -> InMemorySection {
        InMemorySection {
            byte_buf: new_byte_buf(initial),
            cur_size: 0,
        }
    }
}

impl Read for InMemorySection {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let max = self.cur_size as u64 - self.byte_buf.position();
        if buf.len() as u64 > max {
            self.byte_buf.read(&mut buf[..max as usize])
        } else {
            self.byte_buf.read(buf)
        }
    }
}

impl Write for InMemorySection {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let len = self.byte_buf.write(buf)?;
        if self.byte_buf.position() as usize >= self.cur_size {
            self.cur_size = self.byte_buf.position() as usize;
        }
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        self.byte_buf.flush()
    }
}

impl Seek for InMemorySection {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.byte_buf.seek(pos)
    }
}

impl ReadToVec for InMemorySection {
    fn read_to_vec(&mut self) -> Result<Vec<u8>> {
        Ok(self.byte_buf.get_ref().clone())
    }
}

impl SectionData for InMemorySection {
    fn truncate(&mut self, size: usize) -> Result<usize> {
        if size == 0 {
            return Ok(0);
        }
        //CMP is here to ensure no panic is possible!
        self.cur_size -= std::cmp::min(self.cur_size, size);
        if self.byte_buf.position() > self.cur_size as u64 {
            self.byte_buf.set_position(self.cur_size as u64);
        }
        Ok(self.cur_size)
    }

    fn size(&self) -> usize {
        self.cur_size
    }
}

#[cfg(test)]
mod tests {
    use crate::core::data::memory::InMemorySection;
    use crate::core::SectionData;
    use crate::util::traits::ReadFill;
    use std::io::{Seek, SeekFrom, Write};

    #[test]
    fn basic_truncate() {
        let mut data = InMemorySection::new(4);
        data.write_all(b"test").unwrap();
        let new_len = data.truncate(2).unwrap();
        assert_eq!(new_len, 2);
        let mut buf = [0; 4];
        data.seek(SeekFrom::Start(0)).unwrap();
        let len = data.read_fill(&mut buf).unwrap();
        assert_eq!(len, 2);
        assert_eq!(&buf, b"te\0\0");
    }
}
