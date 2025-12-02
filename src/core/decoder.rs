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

//! The BPX decoder.

use std::num::NonZeroU32;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    io,
    io::{Read, Seek, Write},
};

use crate::core::handle::HandleGenerator;
use crate::core::SectionInfo;
use crate::core::{
    compression::{
        Checksum, Crc32Checksum, Inflater, WeakChecksum, XzCompressionMethod,
        ZlibCompressionMethod,
    },
    data::AutoSectionData,
    error::Error,
    header::{
        MainHeader, SectionHeader, Struct, FLAG_CHECK_CRC32, FLAG_CHECK_WEAK, FLAG_COMPRESS_XZ,
        FLAG_COMPRESS_ZLIB,
    },
    section::{SectionEntry, SectionEntry1},
    Result,
};
use crate::util::traits::ReadFill;
use super::header::GetChecksum;

const READ_BLOCK_SIZE: usize = 8192;

pub fn read_section_header_table<T: Read>(
    mut backend: &mut T,
    main_header: &MainHeader,
    checksum: &mut impl Checksum,
    default_compression_threshold: u32,
) -> Result<(HandleGenerator, BTreeMap<NonZeroU32, SectionEntry>)> {
    let mut sections = BTreeMap::new();
    let mut hdl = HandleGenerator::new();

    for i in 0..main_header.section_num {
        let hdl = hdl.next();
        let header = SectionHeader::read(&mut backend)?;
        header.get_checksum(checksum);
        sections.insert(
            hdl.0,
            SectionEntry {
                info: SectionInfo::new(i, header),
                data: RefCell::new(None),
                modified: Cell::new(false),
                entry1: SectionEntry1 {
                    flags: header.flags,
                    threshold: default_compression_threshold,
                },
            },
        );
    }
    Ok((hdl, sections))
}

pub fn load_section1<T: Read + Seek>(
    file: &mut T,
    section: &SectionHeader,
    memory_threshold: u32,
) -> Result<AutoSectionData> {
    let mut data = AutoSectionData::new_with_size(section.size as _, memory_threshold as _)?;
    data.seek(io::SeekFrom::Start(0))?;
    if section.flags & FLAG_CHECK_WEAK != 0 {
        let mut chksum = WeakChecksum::new(0);
        load_section_checked(file, section, &mut data, &mut chksum)?;
        let v = chksum.finish();
        if v != section.chksum {
            return Err(Error::Checksum {
                actual: v,
                expected: section.chksum,
            });
        }
    } else if section.flags & FLAG_CHECK_CRC32 != 0 {
        let mut chksum = Crc32Checksum::new();
        load_section_checked(file, section, &mut data, &mut chksum)?;
        let v = chksum.finish();
        if v != section.chksum {
            return Err(Error::Checksum {
                actual: v,
                expected: section.chksum,
            });
        }
    } else {
        let mut chksum = WeakChecksum::new(0);
        load_section_checked(file, section, &mut data, &mut chksum)?;
    }
    data.seek(io::SeekFrom::Start(0))?;
    Ok(data)
}

fn load_section_checked<TBackend: Read + Seek, TWrite: Write, TChecksum: Checksum>(
    file: &mut TBackend,
    section: &SectionHeader,
    out: TWrite,
    chksum: &mut TChecksum,
) -> Result<()> {
    if section.flags & FLAG_COMPRESS_XZ != 0 {
        load_section_compressed::<XzCompressionMethod, _, _, _>(file, section, out, chksum)?;
    } else if section.flags & FLAG_COMPRESS_ZLIB != 0 {
        load_section_compressed::<ZlibCompressionMethod, _, _, _>(file, section, out, chksum)?;
    } else {
        load_section_uncompressed(file, section, out, chksum)?;
    }
    Ok(())
}

fn load_section_uncompressed<TBackend: Read + Seek, TWrite: Write, TChecksum: Checksum>(
    bpx: &mut TBackend,
    header: &SectionHeader,
    mut output: TWrite,
    chksum: &mut TChecksum,
) -> io::Result<()> {
    let mut idata: [u8; READ_BLOCK_SIZE] = [0; READ_BLOCK_SIZE];
    let mut count: usize = 0;
    let mut remaining: usize = header.size as usize;

    bpx.seek(io::SeekFrom::Start(header.pointer))?;
    while count < header.size as usize {
        let res = bpx.read_fill(&mut idata[0..std::cmp::min(READ_BLOCK_SIZE, remaining)])?;
        output.write_all(&idata[0..res])?;
        chksum.push(&idata[0..res]);
        count += res;
        remaining -= res;
    }
    Ok(())
}

fn load_section_compressed<
    TMethod: Inflater,
    TBackend: Read + Seek,
    TWrite: Write,
    TChecksum: Checksum,
>(
    bpx: &mut TBackend,
    header: &SectionHeader,
    output: TWrite,
    chksum: &mut TChecksum,
) -> Result<()> {
    bpx.seek(io::SeekFrom::Start(header.pointer))?;
    XzCompressionMethod::inflate(bpx, output, header.csize as usize, chksum)?;
    Ok(())
}
