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

use crate::core::data::util::IoReadBuffer;
use std::{
    fs::File,
    io::{Read, Result, Seek, SeekFrom, Write},
};

use crate::core::SectionData;
use crate::util::traits::ReadToVec;

pub struct FileBasedSection {
    data: File,
    buffer: IoReadBuffer,
    stream_pos: u64,
    cur_size: usize,
}

impl FileBasedSection {
    pub fn new(data: File) -> FileBasedSection {
        FileBasedSection {
            data,
            buffer: IoReadBuffer::new(),
            stream_pos: 0,
            cur_size: 0,
        }
    }
}

impl Read for FileBasedSection {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.buffer.read(buf, |block| {
            let max = self.cur_size as u64 - self.stream_pos;
            let len = match block.len() as u64 > max {
                true => self.data.read(&mut block[..max as usize]),
                false => self.data.read(block),
            }?;
            self.stream_pos += len as u64;
            Ok(len)
        })
    }
}

impl Write for FileBasedSection {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let real_pos = self.stream_position()?;
        self.data.seek(SeekFrom::Start(real_pos))?;
        let len = self.data.write(buf)?;
        if real_pos >= self.cur_size as u64 {
            self.cur_size += len;
        }
        self.stream_pos = real_pos + len as u64;
        self.buffer.flush();
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        self.data.flush()
    }
}

impl Seek for FileBasedSection {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.stream_pos = self.data.seek(pos)?;
        self.buffer.flush();
        Ok(self.stream_pos)
    }

    fn stream_position(&mut self) -> Result<u64> {
        Ok(self.stream_pos - self.buffer.inverted_position())
    }
}

impl ReadToVec for FileBasedSection {}

impl SectionData for FileBasedSection {
    fn truncate(&mut self, size: usize) -> Result<usize> {
        if size == 0 {
            return Ok(0);
        }
        //CMP is here to ensure no panic is possible!
        self.cur_size -= std::cmp::min(self.cur_size, size);
        let real_pos = self.stream_position()?;
        if real_pos > self.cur_size as u64 {
            self.data.seek(SeekFrom::Start(self.cur_size as u64))?;
            self.buffer.flush();
        }
        Ok(self.cur_size)
    }

    fn size(&self) -> usize {
        self.cur_size
    }
}

#[cfg(test)]
mod tests {
    use crate::core::data::file::FileBasedSection;
    use crate::core::SectionData;
    use crate::util::traits::ReadFill;
    use std::io::{Seek, SeekFrom, Write};

    #[test]
    fn basic_read_write_seek() {
        let mut data = FileBasedSection::new(tempfile::tempfile().unwrap());
        data.write_all(b"test").unwrap();
        data.seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0; 4];
        let len = data.read_fill(&mut buf).unwrap();
        assert_eq!(len, 4);
        assert_eq!(&buf, b"test");
    }

    #[test]
    fn basic_truncate() {
        let mut data = FileBasedSection::new(tempfile::tempfile().unwrap());
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
