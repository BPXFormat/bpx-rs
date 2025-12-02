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

use std::io::{Read, Seek, SeekFrom, Write};

use tempfile::tempfile;

use crate::core::{
    data::{file::FileBasedSection, memory::InMemorySection},
    SectionData, DEFAULT_MEMORY_THRESHOLD,
};
use crate::util::traits::ReadToVec;

const INIT_BUF_SIZE: usize = 512;

#[allow(clippy::large_enum_variant)] // This is always used behind a Box
enum DynSectionData {
    File(FileBasedSection),
    Memory(InMemorySection),
}

/// Automatic section data implementation.
///
/// *This automatically switches an in-memory section data into a file backed section data
/// when the size of the data exceeds 100Mb.*
pub struct AutoSectionData {
    inner: Box<DynSectionData>,
    memory_threshold: usize,
}

impl Default for AutoSectionData {
    fn default() -> Self {
        Self::new(DEFAULT_MEMORY_THRESHOLD as _)
    }
}

impl AutoSectionData {
    /// Creates a new section data backed by a dynamically sized in-memory buffer.
    pub fn new(memory_threshold: usize) -> AutoSectionData {
        AutoSectionData {
            inner: Box::new(DynSectionData::Memory(InMemorySection::new(512))),
            memory_threshold,
        }
    }

    /// Creates a new section data with a known size limit.
    ///
    /// # Arguments
    ///
    /// * `size`: the size of the new section data.
    /// * `memory_threshold`: the maximum size of a section in memort (RAM) in bytes.
    ///
    /// returns: Result<AutoSectionData, Error>
    ///
    /// # Errors
    ///
    /// This function returns an [Error](std::io::Error) if a file backed section was needed,
    /// given the size constraint, but failed to initialize.
    pub fn new_with_size(size: usize, memory_threshold: usize) -> std::io::Result<AutoSectionData> {
        if size >= memory_threshold {
            let file = FileBasedSection::new(tempfile()?);
            Ok(AutoSectionData {
                inner: Box::new(DynSectionData::File(file)),
                memory_threshold,
            })
        } else {
            Ok(AutoSectionData {
                inner: Box::new(DynSectionData::Memory(InMemorySection::new(size))),
                memory_threshold,
            })
        }
    }

    /// Moves this section from in-memory to file (used to avoid overloading RAM with the section
    /// is getting too big.
    ///
    /// # Safety
    ///
    /// This function causes **undefined behavior** if the section is already a file.
    unsafe fn move_to_file(&mut self) -> std::io::Result<()> {
        let mut file = FileBasedSection::new(tempfile()?);
        match &mut *self.inner {
            DynSectionData::Memory(m) => {
                m.seek(SeekFrom::Start(0))?;
                std::io::copy(m, &mut file)
            },
            //SAFETY: If the section is not an InMemorySection then move_to_file is not supposed to have been called,
            // and that is an unrecoverable internal BPX error.
            DynSectionData::File(_) => std::hint::unreachable_unchecked(),
        }?;
        self.inner = Box::new(DynSectionData::File(file));
        Ok(())
    }

    /// Clears this section data and resets to a default dynamically sized in-memory buffer.
    pub fn clear(&mut self) {
        self.inner = Box::new(DynSectionData::Memory(InMemorySection::new(INIT_BUF_SIZE)))
    }
}

macro_rules! auto_section_delegate {
    ($self: ident, $v: ident => $e: expr) => {
        match &*$self.inner {
            DynSectionData::File($v) => $e,
            DynSectionData::Memory($v) => $e,
        }
    };

    (mut $self: ident, $v: ident => $e: expr) => {
        match &mut *$self.inner {
            DynSectionData::File($v) => $e,
            DynSectionData::Memory($v) => $e,
        }
    };
}

impl Read for AutoSectionData {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        auto_section_delegate!(mut self, v => v.read(buf))
    }
}

impl Write for AutoSectionData {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut *self.inner {
            DynSectionData::File(f) => f.write(buf),
            DynSectionData::Memory(m) => {
                let size = m.write(buf)?;
                if m.size() >= self.memory_threshold {
                    unsafe {
                        self.move_to_file()?;
                    }
                }
                Ok(size)
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        auto_section_delegate!(mut self, v => v.flush())
    }
}

impl Seek for AutoSectionData {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        auto_section_delegate!(mut self, v => v.seek(pos))
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        auto_section_delegate!(mut self, v => v.stream_position())
    }
}

impl ReadToVec for AutoSectionData {
    fn read_to_vec(&mut self) -> std::io::Result<Vec<u8>> {
        auto_section_delegate!(mut self, v => v.read_to_vec())
    }
}

impl SectionData for AutoSectionData {
    fn truncate(&mut self, size: usize) -> std::io::Result<usize> {
        auto_section_delegate!(mut self, v => v.truncate(size))
    }

    fn size(&self) -> usize {
        auto_section_delegate!(self, v => v.size())
    }
}
