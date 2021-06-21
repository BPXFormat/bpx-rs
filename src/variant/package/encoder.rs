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

use std::{
    fs::{metadata, read_dir, File},
    io::Read,
    path::Path,
    string::String
};

use byteorder::{ByteOrder, LittleEndian};

use crate::{
    builder::{Checksum, CompressionMethod, MainHeaderBuilder, SectionHeaderBuilder},
    encoder::{Encoder, IoBackend},
    header::{SectionHeader, SECTION_TYPE_SD, SECTION_TYPE_STRING},
    sd::Object,
    strings::{get_name_from_dir_entry, get_name_from_path, StringSection},
    variant::package::{Architecture, Platform, DATA_SECTION_TYPE},
    Interface,
    Result,
    SectionHandle
};

const DATA_WRITE_BUFFER_SIZE: usize = 8192;
const MIN_DATA_REMAINING_SIZE: usize = DATA_WRITE_BUFFER_SIZE;
const MAX_DATA_SECTION_SIZE: usize = 200000000 - MIN_DATA_REMAINING_SIZE; //200MB

/// Utility to easily generate a [PackageEncoder](crate::bpxp::encoder::PackageEncoder)
pub struct PackageBuilder
{
    architecture: Architecture,
    platform: Platform,
    metadata: Option<Object>,
    type_code: [u8; 2]
}

impl PackageBuilder
{
    /// Creates a new BPX Package builder
    ///
    /// # Returns
    ///
    /// * the new BPX Package builder
    pub fn new() -> PackageBuilder
    {
        return PackageBuilder {
            architecture: Architecture::Any,
            platform: Platform::Any,
            metadata: None,
            type_code: [0x50, 0x48]
        };
    }

    /// Defines the CPU architecture that the package is targeting
    ///
    /// - *By default, no CPU architecture is targeted*
    ///
    /// # Arguments
    ///
    /// * `arch` - the new [Architecture](crate::bpxp::Architecture)
    pub fn with_architecture(mut self, arch: Architecture) -> Self
    {
        self.architecture = arch;
        return self;
    }

    /// Defines the platform that the package is targeting
    ///
    /// - *By default, no platform is targeted*
    ///
    /// # Arguments
    ///
    /// * `platform` - the new [Platform](crate::bpxp::Platform)
    pub fn with_platform(mut self, platform: Platform) -> Self
    {
        self.platform = platform;
        return self;
    }

    /// Defines the metadata for the package
    ///
    /// - *By default, no metadata object is set*
    ///
    /// # Arguments
    ///
    /// * `obj` - the new BPXSD [Object](crate::sd::Object) metadata
    pub fn with_metadata(mut self, obj: Object) -> Self
    {
        self.metadata = Some(obj);
        return self;
    }

    /// Defines the variant of the package
    ///
    /// - *By default, the package variant is 'PK' to identify a package destined for FPKG C++ package manager*
    ///
    /// # Arguments
    ///
    /// * `type_code` - an array with 2 bytes
    pub fn with_variant(mut self, type_code: [u8; 2]) -> Self
    {
        self.type_code = type_code;
        return self;
    }

    /// Builds the corresponding [PackageEncoder](crate::bpxp::encoder::PackageEncoder)
    ///
    /// # Arguments
    ///
    /// * `encoder` - the BPX [Encoder](crate::encoder::Encoder) backend to use
    ///
    /// # Returns
    ///
    /// * the new [PackageEncoder](crate::bpxp::encoder::PackageEncoder) if the operation succeeded
    /// * an [Error](crate::error::Error) in case of system error
    pub fn build<TBackend: IoBackend>(self, encoder: &mut Encoder<TBackend>) -> Result<PackageEncoder<TBackend>>
    {
        let mut type_ext: [u8; 16] = [0; 16];
        match self.architecture {
            Architecture::X86_64 => type_ext[0] = 0x0,
            Architecture::Aarch64 => type_ext[0] = 0x1,
            Architecture::X86 => type_ext[0] = 0x2,
            Architecture::Armv7hl => type_ext[0] = 0x3,
            Architecture::Any => type_ext[0] = 0x4
        }
        match self.platform {
            Platform::Linux => type_ext[1] = 0x0,
            Platform::Mac => type_ext[1] = 0x1,
            Platform::Windows => type_ext[1] = 0x2,
            Platform::Android => type_ext[1] = 0x3,
            Platform::Any => type_ext[1] = 0x4
        }
        type_ext[2] = self.type_code[0];
        type_ext[3] = self.type_code[1];
        let header = MainHeaderBuilder::new()
            .with_type('P' as u8)
            .with_type_ext(type_ext)
            .build();
        encoder.set_main_header(header);
        let strings_header = SectionHeaderBuilder::new()
            .with_checksum(Checksum::Weak)
            .with_compression(CompressionMethod::Zlib)
            .with_type(SECTION_TYPE_STRING)
            .build();
        let strings = encoder.create_section(strings_header)?;
        if let Some(obj) = self.metadata {
            let metadata_header = SectionHeaderBuilder::new()
                .with_checksum(Checksum::Weak)
                .with_compression(CompressionMethod::Zlib)
                .with_type(SECTION_TYPE_SD)
                .build();
            let metadata = encoder.create_section(metadata_header)?;
            obj.write(&mut encoder.open_section(metadata)?)?;
        }
        return Ok(PackageEncoder { strings, encoder });
    }
}

/// Represents a BPX Package encoder
pub struct PackageEncoder<'a, TBackend: IoBackend>
{
    strings: SectionHandle,
    encoder: &'a mut Encoder<TBackend>
}

fn create_data_section_header() -> SectionHeader
{
    let header = SectionHeaderBuilder::new()
        .with_type(DATA_SECTION_TYPE)
        .with_compression(CompressionMethod::Xz)
        .with_checksum(Checksum::Crc32)
        .build();
    return header;
}

impl<'a, TBackend: IoBackend> PackageEncoder<'a, TBackend>
{
    fn write_object<TRead: Read>(&mut self, source: &mut TRead, data_id: SectionHandle) -> Result<bool>
    {
        let data = self.encoder.open_section(data_id)?;
        let mut buf: [u8; DATA_WRITE_BUFFER_SIZE] = [0; DATA_WRITE_BUFFER_SIZE];
        let mut res = source.read(&mut buf)?;

        while res > 0 {
            data.write(&buf[0..res])?;
            if data.size() >= MAX_DATA_SECTION_SIZE
            //Split sections (this is to avoid reaching the 4Gb max)
            {
                return Ok(false);
            }
            res = source.read(&mut buf)?;
        }
        return Ok(true);
    }

    fn pack_file(
        &mut self,
        source: &Path,
        name: String,
        data_id1: SectionHandle,
        strings: &mut StringSection
    ) -> Result<SectionHandle>
    {
        let mut data_id = data_id1;
        let size = metadata(source)?.len();
        let mut fle = File::open(source)?;
        let mut buf: [u8; 12] = [0; 12];

        #[cfg(feature = "debug-log")]
        println!("Writing file {} with {} byte(s)", name, size);
        LittleEndian::write_u64(&mut buf[0..8], size);
        LittleEndian::write_u32(&mut buf[8..12], strings.put(self.encoder, &name)?);
        {
            let data = self.encoder.open_section(data_id)?;
            data.write(&buf)?;
        }
        while !self.write_object(&mut fle, data_id)? {
            data_id = self.encoder.create_section(create_data_section_header())?;
        }
        return Ok(data_id);
    }

    fn pack_dir(
        &mut self,
        source: &Path,
        name: String,
        data_id1: SectionHandle,
        strings: &mut StringSection
    ) -> Result<()>
    {
        let mut data_id = data_id1;
        let entries = read_dir(source)?;

        for rentry in entries {
            let entry = rentry?;
            let mut s = name.clone();
            s.push('/');
            s.push_str(&get_name_from_dir_entry(&entry));
            if entry.file_type()?.is_dir() {
                self.pack_dir(&entry.path(), s, data_id, strings)?
            } else {
                data_id = self.pack_file(&entry.path(), s, data_id, strings)?;
            }
        }
        return Ok(());
    }

    /// Packs a file or folder in this BPXP with the given virtual name
    ///
    /// *this functions prints some information to standard output as a way to debug data compression issues*
    ///
    /// # Arguments
    ///
    /// * `encoder` - the BPX [Encoder](crate::encoder::Encoder) backend to use
    /// * `source` - the source [Path](std::path::Path) to pack
    /// * `vname` - the virtual name for the root source path
    ///
    /// # Returns
    ///
    /// * nothing if the operation succeeded
    /// * an [Error](crate::error::Error) in case of system error
    pub fn pack_vname(&mut self, source: &Path, vname: &str) -> Result<()>
    {
        let mut strings = StringSection::new(self.strings);
        let md = metadata(source)?;
        let data_section = self.encoder.create_section(create_data_section_header())?;
        if md.is_file() {
            self.pack_file(source, String::from(vname), data_section, &mut strings)?;
            return Ok(());
        } else {
            return self.pack_dir(source, String::from(vname), data_section, &mut strings);
        }
    }

    /// Packs a file or folder in this BPXP, automatically computing the virtual name from the source path file name
    ///
    /// *this functions prints some information to standard output as a way to debug data compression issues*
    ///
    /// # Arguments
    ///
    /// * `encoder` - the BPX [Encoder](crate::encoder::Encoder) backend to use
    /// * `source` - the source [Path](std::path::Path) to pack
    ///
    /// # Returns
    ///
    /// * nothing if the operation succeeded
    /// * an [Error](crate::error::Error) in case of system error
    pub fn pack(&mut self, source: &Path) -> Result<()>
    {
        let mut strings = StringSection::new(self.strings);
        let md = metadata(source)?;
        let data_section = self.encoder.create_section(create_data_section_header())?;
        if md.is_file() {
            self.pack_file(source, get_name_from_path(source)?, data_section, &mut strings)?;
            return Ok(());
        } else {
            return self.pack_dir(source, get_name_from_path(source)?, data_section, &mut strings);
        }
    }
}
