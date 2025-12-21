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

//! The BPX encoder.

use std::num::NonZeroU32;
use std::{
    collections::BTreeMap,
    io::{Seek, SeekFrom, Write},
};

use crate::core::header::SectionHeader;
use crate::core::SectionInfo;
use crate::core::{
    compression::{
        Checksum, Crc32Checksum, Deflater, WeakChecksum, XzCompressionMethod, ZlibCompressionMethod,
    },
    error::{Error, OpenError},
    header::{
        GetChecksum, MainHeader, Struct, FLAG_CHECK_CRC32, FLAG_CHECK_WEAK, FLAG_COMPRESS_XZ,
        FLAG_COMPRESS_ZLIB, SIZE_MAIN_HEADER, SIZE_SECTION_HEADER,
    },
    section::SectionEntry,
    Result, SectionData,
};
use crate::util::traits::ReadFill;

const READ_BLOCK_SIZE: usize = 8192;

fn write_sections<T: Write + Seek>(
    mut backend: T,
    sections: &mut BTreeMap<NonZeroU32, SectionEntry>,
    file_start_offset: usize,
    chksum_sht: &mut impl Checksum,
) -> Result<usize> {
    let mut ptr: u64 = file_start_offset as _;
    let mut all_sections_size: usize = 0;

    for (idx, (_handle, section)) in sections.iter_mut().enumerate() {
        //At this point the handle must be valid otherwise sections_in_order is broken
        let data = section
            .data
            .get_mut()
            .as_mut()
            .ok_or(Error::Open(OpenError::SectionNotLoaded))?;
        if data.size() > u32::MAX as usize {
            return Err(Error::Capacity(data.size()));
        }
        let last_section_ptr = data.stream_position()?;
        data.seek(SeekFrom::Start(0))?;
        let flags = section.entry1.get_flags(data.size() as u32);
        let (csize, chksum) = write_section(flags, data, &mut backend)?;
        data.seek(SeekFrom::Start(last_section_ptr))?;
        section.info = SectionInfo::new(
            idx as _,
            SectionHeader {
                pointer: ptr,
                csize: csize as _,
                size: data.size() as _,
                chksum,
                ty: section.info.header().ty,
                flags,
            },
        );
        #[cfg(feature = "debug-log")]
        println!(
            "Writing section #{}: Size = {}, Size after compression = {}, Handle = {}",
            idx,
            section.info.header().size,
            section.info.header().csize,
            _handle
        );
        ptr += csize as u64;
        {
            //Locate section header offset, then directly write section header
            let header_start_offset = SIZE_MAIN_HEADER + (idx * SIZE_SECTION_HEADER);
            backend.seek(SeekFrom::Start(header_start_offset as _))?;
            section.info.header().write(&mut backend)?;
            //Reset file pointer back to the end of the last written section
            backend.seek(SeekFrom::Start(ptr))?;
        }
        section.info.header().get_checksum(chksum_sht);
        all_sections_size += csize;
    }
    Ok(all_sections_size)
}

fn internal_save<T: Write + Seek>(
    mut backend: T,
    sections: &mut BTreeMap<NonZeroU32, SectionEntry>,
    main_header: &mut MainHeader,
) -> Result<()> {
    let file_start_offset =
        SIZE_MAIN_HEADER + (SIZE_SECTION_HEADER * main_header.section_num as usize);
    //Seek to the start of the actual file content
    backend.seek(SeekFrom::Start(file_start_offset as _))?;
    //Write all section data and section headers
    let mut chksum_sht = WeakChecksum::default();
    let all_sections_size =
        write_sections(&mut backend, sections, file_start_offset, &mut chksum_sht)?;
    main_header.file_size = all_sections_size as u64 + file_start_offset as u64;
    main_header.get_checksum(&mut chksum_sht);
    main_header.chksum = chksum_sht.finish();
    //Relocate to the start of the file and write the BPX main header
    backend.seek(SeekFrom::Start(0))?;
    main_header.write(&mut backend)?;
    Ok(())
}

fn write_section_single<T: Write + Seek>(
    mut backend: T,
    sections: &mut BTreeMap<NonZeroU32, SectionEntry>,
    handle: NonZeroU32,
) -> Result<(bool, i64)> {
    let entry = sections.get_mut(&handle).unwrap();
    backend.seek(SeekFrom::Start(entry.info.header().pointer))?;
    let data = entry
        .data
        .get_mut()
        .as_mut()
        .ok_or(Error::Open(OpenError::SectionNotLoaded))?;
    let last_section_ptr = data.stream_position()?;
    let flags = entry.entry1.get_flags(data.size() as u32);
    data.seek(SeekFrom::Start(0))?;
    let (csize, chksum) = write_section(flags, data, &mut backend)?;
    data.seek(SeekFrom::Start(last_section_ptr))?;
    let old = *entry.info.header();
    entry.info.header_mut().csize = csize as u32;
    entry.info.header_mut().size = data.size() as u32;
    entry.info.header_mut().chksum = chksum;
    entry.info.header_mut().flags = flags;
    let diff = entry.info.header().csize as i64 - old.csize as i64;
    Ok((&old != entry.info.header(), diff))
}

pub fn recompute_header_checksum(
    main_header: &mut MainHeader,
    sections: &BTreeMap<NonZeroU32, SectionEntry>,
) {
    let mut chksum_sht = WeakChecksum::default();
    for entry in sections.values() {
        entry.info.header().get_checksum(&mut chksum_sht);
    }
    main_header.get_checksum(&mut chksum_sht);
    main_header.chksum = chksum_sht.finish();
}

fn internal_save_single<T: Write + Seek>(
    mut backend: T,
    sections: &mut BTreeMap<NonZeroU32, SectionEntry>,
    main_header: &mut MainHeader,
    handle: NonZeroU32,
) -> Result<bool> {
    // This function saves only the last section.
    let mut write_main_header = false;
    let (update_sht, diff) = write_section_single(&mut backend, sections, handle)?;
    if update_sht {
        let entry = &sections[&handle];
        let offset_section_header =
            SIZE_MAIN_HEADER as u64 + (SIZE_SECTION_HEADER as u64 * entry.info.index() as u64);
        backend.seek(SeekFrom::Start(offset_section_header))?;
        entry.info.header().write(&mut backend)?;
        write_main_header = true;
    }
    if diff != 0 {
        main_header.file_size = main_header.file_size.wrapping_add(diff as u64);
        write_main_header = true;
    }
    if write_main_header {
        recompute_header_checksum(main_header, sections);
        backend.seek(SeekFrom::Start(0))?;
        main_header.write(&mut backend)?;
    }
    Ok(write_main_header)
}

fn write_section_uncompressed<TWrite: Write, TChecksum: Checksum>(
    section: &mut dyn SectionData,
    out: &mut TWrite,
    chksum: &mut TChecksum,
) -> Result<usize> {
    let mut idata: [u8; READ_BLOCK_SIZE] = [0; READ_BLOCK_SIZE];
    let mut count: usize = 0;
    while count < section.size() {
        let res = section.read_fill(&mut idata)?;
        out.write_all(&idata[0..res])?;
        chksum.push(&idata[0..res]);
        count += res;
    }
    section.flush()?;
    Ok(section.size())
}

fn write_section_compressed<TMethod: Deflater, TWrite: Write, TChecksum: Checksum>(
    mut section: &mut dyn SectionData,
    out: &mut TWrite,
    chksum: &mut TChecksum,
) -> Result<usize> {
    let size = section.size();
    let csize = TMethod::deflate(&mut section, out, size, chksum)?;
    Ok(csize)
}

fn write_section_checked<TWrite: Write, TChecksum: Checksum>(
    flags: u8,
    section: &mut dyn SectionData,
    out: &mut TWrite,
    chksum: &mut TChecksum,
) -> Result<usize> {
    if flags & FLAG_COMPRESS_XZ != 0 {
        write_section_compressed::<XzCompressionMethod, _, _>(section, out, chksum)
    } else if flags & FLAG_COMPRESS_ZLIB != 0 {
        write_section_compressed::<ZlibCompressionMethod, _, _>(section, out, chksum)
    } else {
        write_section_uncompressed(section, out, chksum)
    }
}

fn write_section<TWrite: Write>(
    flags: u8,
    section: &mut dyn SectionData,
    out: &mut TWrite,
) -> Result<(usize, u32)> {
    if flags & FLAG_CHECK_CRC32 != 0 {
        let mut chksum = Crc32Checksum::new();
        let size = write_section_checked(flags, section, out, &mut chksum)?;
        Ok((size, chksum.finish()))
    } else if flags & FLAG_CHECK_WEAK != 0 {
        let mut chksum = WeakChecksum::default();
        let size = write_section_checked(flags, section, out, &mut chksum)?;
        Ok((size, chksum.finish()))
    } else {
        let mut chksum = WeakChecksum::default();
        let size = write_section_checked(flags, section, out, &mut chksum)?;
        Ok((size, 0))
    }
}

#[derive(Eq, PartialEq)]
pub enum SaveMode {
    Regenerate,
    MainHeaderOnly,
    PatchMultipleSections,
    PatchSingleSection(NonZeroU32),
}

pub struct Encoder<'a> {
    pub main_header: &'a mut MainHeader,
    pub sections: &'a mut BTreeMap<NonZeroU32, SectionEntry>,
    pub main_header_modified: &'a mut bool,
    pub table_modified: &'a mut bool,
    pub table_count: u32,
    pub mode: SaveMode,
}

impl Encoder<'_> {
    fn get_modified_sections(&self) -> Vec<NonZeroU32> {
        // Returns a list of modified sections.
        self.sections
            .iter()
            .filter(|(_, entry)| entry.modified.get())
            .map(|(handle, _)| *handle)
            .collect()
    }

    fn patch_main_header_if_needed<B: Write + Seek>(
        &mut self,
        mut backend: B,
        was_written: bool,
    ) -> Result<()> {
        if was_written {
            *self.main_header_modified = false;
            Ok(())
        } else if *self.main_header_modified {
            // If only main header changed -> write only main header.
            *self.main_header_modified = false;
            recompute_header_checksum(self.main_header, self.sections);
            backend.seek(SeekFrom::Start(0))?;
            self.main_header.write(&mut backend)?;
            Ok(())
        } else {
            Ok(())
        }
    }

    fn patch_modified_sections<B: Write + Seek>(&mut self, mut backend: B) -> Result<bool> {
        let mut main_header = false;
        for section in self.get_modified_sections() {
            if internal_save_single(&mut backend, self.sections, self.main_header, section)? {
                main_header = true;
            }
        }
        Ok(main_header)
    }

    pub fn run<T: Write + Seek>(mut self, mut backend: T) -> Result<()> {
        match self.mode {
            SaveMode::Regenerate => {
                self.main_header.section_num = self.table_count;
                internal_save(backend, self.sections, self.main_header)?;
                *self.main_header_modified = false;
                *self.table_modified = false;
                Ok(())
            }
            SaveMode::MainHeaderOnly => {
                // No sections have changed and the table didn't change; might have nothing to do.
                self.patch_main_header_if_needed(backend, false)
            }
            SaveMode::PatchMultipleSections => {
                //Multiple sections have changed.
                let flag = self.patch_modified_sections(&mut backend)?;
                self.patch_main_header_if_needed(backend, flag)
            }
            SaveMode::PatchSingleSection(section) => {
                //A single section has changed.
                let flag =
                    internal_save_single(&mut backend, self.sections, self.main_header, section)?;
                self.patch_main_header_if_needed(backend, flag)
            }
        }
    }
}
