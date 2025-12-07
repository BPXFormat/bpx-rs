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

//! Declarations for basic constants and low-level file headers.

use std::io;

use bytesutil::{ByteBuf, ReadBytes, StaticByteBuf, WriteBytes};

use crate::{
    core::{
        error::Error,
        options::{Checksum, CompressionMethod},
    },
    util::garraylen::*,
    util::RecoverableError,
};
use crate::util::traits::ReadFill;

/// Represents a serializable and deserializable byte structure in a BPX.
pub trait Struct<const S: usize> {
    /// The output of from_bytes.
    ///
    /// *This is to allow returning additional values specific to some structures.*
    type Output;

    /// The type of error to return if this structure failed to read.
    ///
    /// *Must be constructable from io::Error to satisfy the Read function*
    type Error: From<io::Error>;

    /// Creates a new empty structure.
    fn new() -> Self;

    /// Attempts to read a structure from an IO backend.
    ///
    /// # Arguments
    ///
    /// * `reader`: the IO backend to read from.
    ///
    /// returns: Result<(u32, MainHeader), Self::Error>
    ///
    /// # Errors
    ///
    /// Returns an error if the data could not be read from the IO backend or if
    /// the structure is corrupted.
    ///
    /// # Examples
    ///
    /// ```should_panic
    /// use bpx::core::header::{MainHeader, SIZE_MAIN_HEADER, Struct};
    ///
    /// let mut corrupted: [u8; SIZE_MAIN_HEADER] = [0; SIZE_MAIN_HEADER];
    /// MainHeader::read(&mut corrupted.as_ref()).unwrap();
    /// ```
    fn read<TReader: io::Read>(mut reader: TReader) -> Result<Self::Output, Self::Error> {
        let mut buffer: [u8; S] = [0; S];
        let len = reader.read_fill(&mut buffer)?;
        if len != S {
            if let Some(err) = Self::error_buffer_size() {
                return Err(err);
            }
        }
        Self::from_bytes(buffer)
    }

    /// Returns the error to return when the reader did not read a full buffer.
    ///
    /// **Return None in this function to indicate that this is not an error.**
    fn error_buffer_size() -> Option<Self::Error>;

    /// Attempts to read a structure from a fixed size byte array.
    ///
    /// # Arguments
    ///
    /// * `buffer`: the fixed size byte array to read from.
    ///
    /// returns: Result<Self::Output, Self::Error>
    fn from_bytes(buffer: [u8; S]) -> Result<Self::Output, Self::Error>;

    /// Converts this structure to a fixed size byte array.
    fn to_bytes(&self) -> [u8; S];

    /// Attempts to write this structure to an IO backend.
    ///
    /// # Arguments
    ///
    /// * `writer`: the IO backend to write to.
    ///
    /// returns: Result<(), std::io::Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error](io::Error) if the data could not be
    /// written to the IO backend.
    fn write<TWriter: io::Write>(&self, writer: &mut TWriter) -> io::Result<()> {
        let buf = self.to_bytes();
        writer.write_all(&buf)?;
        Ok(())
    }
}

/// Represents a byte structure with support for checksum.
pub trait GetChecksum<const D: usize>
where
    Self: Struct<D>,
{
    /// Computes the checksum for this header.
    fn get_checksum(&self, checksum: &mut impl super::compression::Checksum) {
        let buf = self.to_bytes();
        checksum.push(&buf);
    }
}

/// The size in bytes of the BPX Main Header.
pub const SIZE_MAIN_HEADER: usize = 40;

/// The size in bytes of a BPX Section Header.
pub const SIZE_SECTION_HEADER: usize = 24;

/// XZ section compression enable flag.
pub const FLAG_COMPRESS_XZ: u8 = 0x2;

/// Section weak checksum enable flag.
pub const FLAG_CHECK_WEAK: u8 = 0x8;

/// ZLIB section compression enable flag.
pub const FLAG_COMPRESS_ZLIB: u8 = 0x1;

/// Section CRC32 checksum enable flag.
pub const FLAG_CHECK_CRC32: u8 = 0x4;

/// The standard variant for a BPX Strings section.
pub const SECTION_TYPE_STRING: u8 = 0xFF;

/// The standard variant for a BPX Structured Data section.
pub const SECTION_TYPE_SD: u8 = 0xFE;

/// The standard variant for a BPX Table section.
pub const SECTION_TYPE_TABLE: u8 = 0xFD;

/// The BPX version this crate supports.
pub const BPX_CURRENT_VERSION: u32 = 0x2;

/// The values allowed for the version field in BPX main header.
pub const KNOWN_VERSIONS: &[u32] = &[0x1, 0x2];

/// The BPX Main Header.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct MainHeader {
    /// BPX signature.
    ///
    /// Offset: +0
    pub signature: [u8; 3],

    /// Type byte.
    ///
    /// Offset: +3
    pub ty: u8,

    /// Weak checksum of all headers.
    ///
    /// Offset: +4
    pub chksum: u32,

    /// Total size of BPX in bytes.
    ///
    /// Offset: +8
    pub file_size: u64,

    /// Number of sections.
    ///
    /// Offset: +16
    pub section_num: u32,

    /// Version of BPX.
    ///
    /// Offset: +20
    pub version: u32,

    /// Extended Type Information.
    ///
    /// Offset: +24
    pub type_ext: StaticByteBuf<16>,
}

impl Struct<SIZE_MAIN_HEADER> for MainHeader {
    type Output = MainHeader;
    type Error = RecoverableError<Self::Output, Error>;

    fn new() -> Self {
        MainHeader {
            signature: *b"BPX",                 //+0
            ty: 0,                              //+3
            chksum: 0,                          //+4
            file_size: SIZE_MAIN_HEADER as u64, //+8
            section_num: 0,                     //+16
            version: BPX_CURRENT_VERSION,       //+20
            type_ext: ByteBuf::default(),
        }
    }

    fn error_buffer_size() -> Option<Self::Error> {
        Some(Error::Truncated.into())
    }

    fn from_bytes(buffer: [u8; SIZE_MAIN_HEADER]) -> Result<Self::Output, Self::Error> {
        let head = MainHeader {
            signature: extract_slice(&buffer, 0),
            ty: buffer[3],
            chksum: u32::read_bytes_le(&buffer[4..8]),
            file_size: u64::read_bytes_le(&buffer[8..16]),
            section_num: u32::read_bytes_le(&buffer[16..20]),
            version: u32::read_bytes_le(&buffer[20..24]),
            type_ext: ByteBuf::new(extract_slice(&buffer, 24)),
        };
        if &head.signature != b"BPX" {
            return Err(RecoverableError::new(
                Error::BadSignature(head.signature),
                head,
            ));
        }
        if !KNOWN_VERSIONS.contains(&head.version) {
            return Err(RecoverableError::new(Error::BadVersion(head.version), head));
        }
        Ok(head)
    }

    fn to_bytes(&self) -> [u8; SIZE_MAIN_HEADER] {
        let mut block: [u8; SIZE_MAIN_HEADER] = [0; SIZE_MAIN_HEADER];
        block[0] = self.signature[0];
        block[1] = self.signature[1];
        block[2] = self.signature[2];
        block[3] = self.ty;
        self.chksum.write_bytes_le(&mut block[4..8]);
        self.file_size.write_bytes_le(&mut block[8..16]);
        self.section_num.write_bytes_le(&mut block[16..20]);
        self.version.write_bytes_le(&mut block[20..24]);
        block[24..40].copy_from_slice(self.type_ext.as_ref());
        block
    }
}

impl GetChecksum<SIZE_MAIN_HEADER> for MainHeader {
    fn get_checksum(&self, checksum: &mut impl super::compression::Checksum) {
        let buf = self.to_bytes();
        checksum.push(&buf[..4]);
        checksum.push(&buf[8..]);
    }
}

/// The BPX Section Header.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct SectionHeader {
    /// Data pointer.
    ///
    /// Offset: +0
    pub pointer: u64,

    /// Size in bytes after compression.
    ///
    /// Offset: +8
    pub csize: u32,

    /// Size in bytes before compression.
    ///
    /// Offset: +12
    pub size: u32,

    /// Data checksum.
    ///
    /// Offset: +16
    pub chksum: u32,

    /// Type byte.
    ///
    /// Offset: +20
    pub ty: u8,

    /// Flags (see FLAG_* constants).
    ///
    /// Offset: +21
    pub flags: u8,
}

impl Struct<SIZE_SECTION_HEADER> for SectionHeader {
    type Output = SectionHeader;
    type Error = Error;

    fn new() -> Self {
        SectionHeader {
            pointer: 0, //+0
            csize: 0,   //+8
            size: 0,    //+12
            chksum: 0,  //+16
            ty: 0,      //+20
            flags: 0,   //+21
        }
    }

    fn error_buffer_size() -> Option<Self::Error> {
        Some(Error::Truncated)
    }

    fn from_bytes(buffer: [u8; SIZE_SECTION_HEADER]) -> Result<Self::Output, Self::Error> {
        Ok(SectionHeader {
            pointer: u64::read_bytes_le(&buffer[0..8]),
            csize: u32::read_bytes_le(&buffer[8..12]),
            size: u32::read_bytes_le(&buffer[12..16]),
            chksum: u32::read_bytes_le(&buffer[16..20]),
            ty: buffer[20],
            flags: buffer[21],
        })
    }

    fn to_bytes(&self) -> [u8; SIZE_SECTION_HEADER] {
        let mut block: [u8; SIZE_SECTION_HEADER] = [0; SIZE_SECTION_HEADER];
        self.pointer.write_bytes_le(&mut block[0..8]);
        self.csize.write_bytes_le(&mut block[8..12]);
        self.size.write_bytes_le(&mut block[12..16]);
        self.chksum.write_bytes_le(&mut block[16..20]);
        block[20] = self.ty;
        block[21] = self.flags;
        block
    }
}

impl GetChecksum<SIZE_SECTION_HEADER> for SectionHeader {}

impl SectionHeader {
    /// Checks if this section is huge (greater than 100Mb).
    pub fn is_huge(&self) -> bool {
        self.size > 100000000
    }

    /// Extracts compression information from this section.
    pub fn compression(&self) -> Option<(CompressionMethod, u32)> {
        if self.flags & FLAG_COMPRESS_ZLIB != 0 {
            Some((CompressionMethod::Zlib, self.csize))
        } else if self.flags & FLAG_COMPRESS_XZ != 0 {
            Some((CompressionMethod::Xz, self.csize))
        } else {
            None
        }
    }

    /// Extracts checksum information from this section.
    pub fn checksum(&self) -> Option<Checksum> {
        if self.flags & FLAG_CHECK_WEAK != 0 {
            Some(Checksum::Weak)
        } else if self.flags & FLAG_CHECK_CRC32 != 0 {
            Some(Checksum::Crc32)
        } else {
            None
        }
    }
}
