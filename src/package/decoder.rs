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

use crate::util::table::NamedItemTable;
use crate::{
    core::{header::Struct, Container, Handle},
    package::{
        error::{Error, InvalidCodeContext},
        object::ObjectHeader,
        Architecture, Platform, Result,
    },
};

const DATA_READ_BUFFER_SIZE: usize = 8192;

fn load_from_section<T: Read + Seek, W: Write>(
    container: &Container<T>,
    handle: Handle,
    offset: u32,
    size: u32,
    out: &mut W,
) -> Result<u32> {
    let mut len = 0;
    let mut buf: [u8; DATA_READ_BUFFER_SIZE] = [0; DATA_READ_BUFFER_SIZE];
    let mut section = container.sections().load(handle)?;

    section.seek(SeekFrom::Start(offset as u64))?;
    while len < size {
        let s = std::cmp::min(size - len, DATA_READ_BUFFER_SIZE as u32);
        // Read is enough as Sections are guaranteed to fill the buffer as much as possible
        let val = section.read(&mut buf[0..s as usize])?;
        len += val as u32;
        out.write_all(&buf[0..val])?;
    }
    Ok(len)
}

pub fn unpack_object<T: Read + Seek, W: Write>(
    container: &Container<T>,
    obj: &ObjectHeader,
    mut out: W,
) -> Result<u64> {
    let mut section_id = obj.start;
    let mut offset = obj.offset;
    let mut len = obj.size;

    while len > 0 {
        let handle = match container.sections().find_by_index(section_id) {
            Some(i) => i,
            None => break,
        };
        let section = container.sections()[handle].header();
        let remaining_section_size = section.size - offset;
        let val = load_from_section(
            container,
            handle,
            offset,
            std::cmp::min(remaining_section_size as u64, len) as u32,
            &mut out,
        )?;
        len -= val as u64;
        offset = 0;
        section_id += 1;
    }
    Ok(obj.size)
}

pub fn read_object_table<T: Read + Seek>(
    container: &Container<T>,
    object_table: Handle,
) -> Result<NamedItemTable<ObjectHeader>> {
    let sections = container.sections();
    let count = sections[object_table].header().size / 20;
    let mut v = Vec::with_capacity(count as _);
    let mut data = sections.load(object_table)?;

    for _ in 0..count {
        let header = ObjectHeader::read(&mut *data)?;
        v.push(header);
    }
    Ok(NamedItemTable::with_list(v))
}

pub fn get_arch_platform_from_code(acode: u8, pcode: u8) -> Result<(Architecture, Platform)> {
    let arch;
    let platform;

    match acode {
        0x0 => arch = Architecture::X86_64,
        0x1 => arch = Architecture::Aarch64,
        0x2 => arch = Architecture::X86,
        0x3 => arch = Architecture::Armv7hl,
        0x4 => arch = Architecture::Any,
        _ => {
            return Err(Error::InvalidCode {
                context: InvalidCodeContext::Arch,
                code: acode,
            })
        }
    }
    match pcode {
        0x0 => platform = Platform::Linux,
        0x1 => platform = Platform::Mac,
        0x2 => platform = Platform::Windows,
        0x3 => platform = Platform::Android,
        0x4 => platform = Platform::Any,
        _ => {
            return Err(Error::InvalidCode {
                context: InvalidCodeContext::Platform,
                code: pcode,
            })
        }
    }
    Ok((arch, platform))
}
