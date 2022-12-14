// Copyright (c) 2021, BlockProject 3D
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

use std::{
    fs::File,
    io,
    io::{Read, Seek, Write}
};

use crate::{
    compression::{Checksum, Crc32Checksum, Deflater, WeakChecksum, XzCompressionMethod, ZlibCompressionMethod},
    error::Error,
    header::{
        MainHeader,
        SectionHeader,
        FLAG_CHECK_CRC32,
        FLAG_CHECK_WEAK,
        FLAG_COMPRESS_XZ,
        FLAG_COMPRESS_ZLIB,
        SIZE_MAIN_HEADER,
        SIZE_SECTION_HEADER
    },
    section::{new_section_data, SectionData},
    Interface,
    Result,
    SectionHandle
};

const READ_BLOCK_SIZE: usize = 8192;

/// Represents the IO backend for a BPX encoder.
pub trait IoBackend: io::Write
{
}
impl<T: io::Write> IoBackend for T {}

/// The BPX encoder.
pub struct Encoder<TBackend: IoBackend>
{
    main_header: MainHeader,
    sections: Vec<SectionHeader>,
    sections_data: Vec<Box<dyn SectionData>>,
    file: TBackend
}

impl<TBackend: IoBackend> Encoder<TBackend>
{
    /// Creates a new BPX encoder.
    ///
    /// # Arguments
    ///
    /// * `file`: An [IoBackend](self::IoBackend) to use for reading the data.
    ///
    /// returns: Result<Encoder<TBackend>, Error>
    pub fn new(file: TBackend) -> Result<Encoder<TBackend>>
    {
        return Ok(Encoder {
            main_header: MainHeader::new(),
            sections: Vec::new(),
            sections_data: Vec::new(),
            file
        });
    }

    /// Sets the BPX Main Header.
    ///
    /// # Arguments
    ///
    /// * `main_header`: the new [MainHeader](crate::header::MainHeader).
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::builder::MainHeaderBuilder;
    /// use bpx::encoder::Encoder;
    /// use bpx::Interface;
    ///
    /// let mut encoder = Encoder::new(Vec::<u8>::new()).unwrap();
    /// encoder.set_main_header(MainHeaderBuilder::new().with_type(1).build());
    /// assert_eq!(encoder.get_main_header().btype, 1);
    /// ```
    pub fn set_main_header(&mut self, main_header: MainHeader)
    {
        self.main_header = main_header;
    }

    /// Creates a new section in the BPX
    ///
    /// # Arguments
    ///
    /// * `header`: the [SectionHeader](crate::header::SectionHeader) of the new section.
    ///
    /// returns: Result<SectionHandle, Error>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::builder::MainHeaderBuilder;
    /// use bpx::encoder::Encoder;
    /// use bpx::header::SectionHeader;
    /// use bpx::Interface;
    ///
    /// let mut encoder = Encoder::new(Vec::<u8>::new()).unwrap();
    /// assert_eq!(encoder.get_main_header().section_num, 0);
    /// encoder.create_section(SectionHeader::new());
    /// assert_eq!(encoder.get_main_header().section_num, 1);
    /// ```
    pub fn create_section(&mut self, header: SectionHeader) -> Result<SectionHandle>
    {
        self.main_header.section_num += 1;
        let section = create_section(&header)?;
        self.sections.push(header);
        let r = self.sections.len() - 1;
        self.sections_data.push(section);
        return Ok(SectionHandle(r));
    }

    fn write_sections(&mut self) -> Result<(File, u32, usize)>
    {
        let mut all_sections_size: usize = 0;
        let mut chksum_sht: u32 = 0;
        let mut ptr: u64 = SIZE_MAIN_HEADER as u64 + (self.sections.len() as u64 * SIZE_SECTION_HEADER as u64);
        let mut f = tempfile::tempfile()?;

        for i in 0..self.sections.len() {
            if self.sections_data[i].size() > u32::MAX as usize {
                return Err(Error::Capacity(self.sections_data[i].size()));
            }
            self.sections_data[i].seek(io::SeekFrom::Start(0))?;
            let flags = get_flags(&self.sections[i], self.sections_data[i].size() as u32);
            let (csize, chksum) = write_section(flags, self.sections_data[i].as_mut(), &mut f)?;
            self.sections[i].csize = csize as u32;
            self.sections[i].size = self.sections_data[i].size() as u32;
            self.sections[i].chksum = chksum;
            self.sections[i].flags = flags;
            self.sections[i].pointer = ptr;
            #[cfg(feature = "debug-log")]
            println!(
                "Writing section #{}: Size = {}, Size after compression = {}",
                i, self.sections[i].size, self.sections[i].csize
            );
            ptr += csize as u64;
            chksum_sht += self.sections[i].get_checksum();
            all_sections_size += csize;
        }
        return Ok((f, chksum_sht, all_sections_size));
    }

    fn write_data_file(&mut self, fle: &mut File, all_sections_size: usize) -> Result<()>
    {
        let mut idata: [u8; 8192] = [0; 8192];
        let mut count: usize = 0;

        fle.seek(io::SeekFrom::Start(0))?;
        while count < all_sections_size {
            let res = fle.read(&mut idata)?;
            self.file.write(&idata[0..res])?;
            count += res;
        }
        return Ok(());
    }

    /// Writes all sections to the underlying IO backend.
    ///
    /// **This function prints some information to standard output as a way
    /// to debug data compression issues unless the `debug-log` feature
    /// is disabled.**
    ///
    /// # Errors
    ///
    /// An [Error](crate::error::Error) is returned if some data could
    /// not be written.
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::encoder::Encoder;
    ///
    /// let mut encoder = Encoder::new(Vec::<u8>::new()).unwrap();
    /// encoder.save();
    /// //TODO: Finish once Encoder can be consumed back into its IO Backend
    /// ```
    pub fn save(&mut self) -> Result<()>
    {
        let (mut main_data, chksum_sht, all_sections_size) = self.write_sections()?;

        self.main_header.file_size =
            all_sections_size as u64 + (self.sections.len() * SIZE_SECTION_HEADER) as u64 + SIZE_MAIN_HEADER as u64;
        self.main_header.chksum = chksum_sht + self.main_header.get_checksum();
        self.main_header.write(&mut self.file)?;
        for v in &self.sections {
            v.write(&mut self.file)?;
        }
        self.write_data_file(&mut main_data, all_sections_size)?;
        return Ok(());
    }
}

impl<TBackend: IoBackend> Interface for Encoder<TBackend>
{
    fn find_section_by_type(&self, btype: u8) -> Option<SectionHandle>
    {
        for i in 0..self.sections.len() {
            if self.sections[i].btype == btype {
                return Some(SectionHandle(i));
            }
        }
        return None;
    }

    fn find_all_sections_of_type(&self, btype: u8) -> Vec<SectionHandle>
    {
        let mut v = Vec::new();

        for i in 0..self.sections.len() {
            if self.sections[i].btype == btype {
                v.push(SectionHandle(i));
            }
        }
        return v;
    }

    fn find_section_by_index(&self, index: u32) -> Option<SectionHandle>
    {
        if let Some(_) = self.sections.get(index as usize) {
            return Some(SectionHandle(index as _));
        }
        return None;
    }

    fn get_section_header(&self, handle: SectionHandle) -> &SectionHeader
    {
        return &self.sections[handle.0];
    }

    fn get_section_index(&self, handle: SectionHandle) -> u32
    {
        return handle.0 as u32;
    }

    fn open_section(&mut self, handle: SectionHandle) -> Result<&mut dyn SectionData>
    {
        return Ok(self.sections_data[handle.0].as_mut());
    }

    fn get_main_header(&self) -> &MainHeader
    {
        return &self.main_header;
    }
}

fn get_flags(header: &SectionHeader, size: u32) -> u8
{
    let mut flags = 0;
    if header.flags & FLAG_CHECK_WEAK != 0 {
        flags |= FLAG_CHECK_WEAK;
    } else if header.flags & FLAG_CHECK_CRC32 != 0 {
        flags |= FLAG_CHECK_CRC32;
    }
    if header.flags & FLAG_COMPRESS_XZ != 0 && size > header.csize {
        flags |= FLAG_COMPRESS_XZ;
    } else if header.flags & FLAG_COMPRESS_ZLIB != 0 && size > header.csize {
        flags |= FLAG_COMPRESS_ZLIB;
    }
    return flags;
}

fn create_section(header: &SectionHeader) -> Result<Box<dyn SectionData>>
{
    if header.size == 0 {
        let mut section = new_section_data(None)?;
        section.seek(io::SeekFrom::Start(0))?;
        return Ok(section);
    } else {
        let mut section = new_section_data(Some(header.size))?;
        section.seek(io::SeekFrom::Start(0))?;
        return Ok(section);
    }
}

fn write_section_uncompressed<TWrite: Write, TChecksum: Checksum>(
    section: &mut dyn SectionData,
    out: &mut TWrite,
    chksum: &mut TChecksum
) -> Result<usize>
{
    let mut idata: [u8; READ_BLOCK_SIZE] = [0; READ_BLOCK_SIZE];
    let mut count: usize = 0;
    while count < section.size() as usize {
        let res = section.read(&mut idata)?;
        out.write(&idata[0..res])?;
        chksum.push(&idata[0..res]);
        count += res;
    }
    section.flush()?;
    return Ok(section.size());
}

fn write_section_compressed<TMethod: Deflater, TWrite: Write, TChecksum: Checksum>(
    mut section: &mut dyn SectionData,
    out: &mut TWrite,
    chksum: &mut TChecksum
) -> Result<usize>
{
    let size = section.size();
    let csize = TMethod::deflate(&mut section, out, size, chksum)?;
    return Ok(csize);
}

fn write_section_checked<TWrite: Write, TChecksum: Checksum>(
    flags: u8,
    section: &mut dyn SectionData,
    out: &mut TWrite,
    chksum: &mut TChecksum
) -> Result<usize>
{
    if flags & FLAG_COMPRESS_XZ != 0 {
        return write_section_compressed::<XzCompressionMethod, _, _>(section, out, chksum);
    } else if flags & FLAG_COMPRESS_ZLIB != 0 {
        return write_section_compressed::<ZlibCompressionMethod, _, _>(section, out, chksum);
    } else {
        return write_section_uncompressed(section, out, chksum);
    }
}

fn write_section<TWrite: Write>(flags: u8, section: &mut dyn SectionData, out: &mut TWrite) -> Result<(usize, u32)>
{
    if flags & FLAG_CHECK_CRC32 != 0 {
        let mut chksum = Crc32Checksum::new();
        let size = write_section_checked(flags, section, out, &mut chksum)?;
        return Ok((size, chksum.finish()));
    } else if flags & FLAG_CHECK_WEAK != 0 {
        let mut chksum = WeakChecksum::new();
        let size = write_section_checked(flags, section, out, &mut chksum)?;
        return Ok((size, chksum.finish()));
    } else {
        let mut chksum = WeakChecksum::new();
        let size = write_section_checked(flags, section, out, &mut chksum)?;
        return Ok((size, 0));
    }
}
