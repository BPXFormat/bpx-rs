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

//! Contains public IO traits to be used by other modules of BPX as well as external codes.

use std::io::Result;
use std::io::Write;

pub use bytesutil::ReadFill;
pub use bytesutil::ReadToVec;

/// Shift direction and length for shifting IO streams.
pub enum ShiftTo {
    /// Shift to the left.
    Left(u64),

    /// Shift to the right.
    Right(u64),
}

/// Represents IO streams with support for byte shifting.
pub trait Shift: Write {
    /// Shifts all bytes after cursor, in the section, to the left or to the right.
    ///
    /// If this operation fails the data in the section may appear partially shifted.
    ///
    /// # Arguments
    ///
    /// * `shift`: the shift direction and length.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Errors
    ///
    /// An [Error](std::io::Error) is returned if a read, write or seek operation has failed.
    fn shift(&mut self, pos: ShiftTo) -> Result<()>;

    /// Appends new bytes at the current cursor position in the IO stream.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Arguments
    ///
    /// * `buf`: the buffer of bytes to write.
    ///
    /// returns: Result<usize, Error>
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer could not be appended at the current cursor position.
    fn write_append(&mut self, buf: &[u8]) -> Result<usize> {
        self.shift(ShiftTo::Right(buf.len() as u64))?;
        self.write(buf)
    }
}
