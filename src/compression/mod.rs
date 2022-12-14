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

use std::io::{Read, Write};

use crate::Result;

mod crc32chksum;
mod weakchksum;
mod xz;
mod zlib;

pub use crc32chksum::Crc32Checksum;
pub use weakchksum::WeakChecksum;
pub use xz::XzCompressionMethod;
pub use zlib::ZlibCompressionMethod;

pub trait Checksum
{
    fn push(&mut self, buffer: &[u8]);
    fn finish(self) -> u32;
}

pub trait Inflater
{
    fn inflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
        input: &mut TRead,
        output: &mut TWrite,
        deflated_size: usize,
        chksum: &mut TChecksum
    ) -> Result<()>;
}

pub trait Deflater
{
    fn deflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
        input: &mut TRead,
        output: &mut TWrite,
        inflated_size: usize,
        chksum: &mut TChecksum
    ) -> Result<usize>;
}
