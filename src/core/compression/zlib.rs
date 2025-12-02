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

use std::io::{Read, Write};

use libz_sys::{
    deflate, deflateEnd, deflateInit_, inflate, inflateEnd, inflateInit_, z_stream, Z_DATA_ERROR,
    Z_DEFAULT_COMPRESSION, Z_FINISH, Z_MEM_ERROR, Z_NEED_DICT, Z_NO_FLUSH, Z_OK, Z_STREAM_ERROR,
    Z_VERSION_ERROR,
};

use crate::core::{
    compression::{Checksum, Deflater, Inflater},
    error::{DeflateError, InflateError},
};
use crate::util::traits::ReadFill;

const ENCODER_BUF_SIZE: usize = 8192;
const DECODER_BUF_SIZE: usize = ENCODER_BUF_SIZE * 2;

// Needed to bypass rust new "feature" to prevent users from using std::mem::zeroed() on UB types.
// Because this z_stream struct is repr(C) rust must guarantee ABI compatibility with C.
// That is must use pointers for function pointer. If it doesn't do this anymore, then this will cause UB in low-level C code.
// Will obviously fail on platforms/architectures not using 0 to represent NULL pointers.
unsafe fn zstream_zeroed() -> z_stream {
    let arr: [u8; std::mem::size_of::<z_stream>()] = [0; std::mem::size_of::<z_stream>()];
    std::mem::transmute(arr)
}

fn new_encoder() -> Result<z_stream, DeflateError> {
    unsafe {
        let mut stream: z_stream = zstream_zeroed();
        let err = deflateInit_(
            &mut stream as _,
            Z_DEFAULT_COMPRESSION,
            "1.1.3".as_ptr() as _,
            std::mem::size_of::<z_stream>() as _,
        );
        if err == Z_OK {
            return Ok(stream);
        }
        match err {
            Z_MEM_ERROR => Err(DeflateError::Memory),
            Z_STREAM_ERROR => Err(DeflateError::Unsupported("compression level")),
            Z_VERSION_ERROR => Err(DeflateError::Unsupported("version")),
            _ => Err(DeflateError::Unknown),
        }
    }
}

fn new_decoder() -> Result<z_stream, InflateError> {
    unsafe {
        let mut stream: z_stream = zstream_zeroed();
        let err = inflateInit_(
            &mut stream as _,
            "1.1.3".as_ptr() as _,
            std::mem::size_of::<z_stream>() as _,
        );
        if err == Z_OK {
            return Ok(stream);
        }
        match err {
            Z_MEM_ERROR => Err(InflateError::Memory),
            Z_DATA_ERROR => Err(InflateError::Data),
            Z_VERSION_ERROR => Err(InflateError::Unsupported("version")),
            _ => Err(InflateError::Unknown),
        }
    }
}

fn do_deflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
    stream: &mut z_stream,
    mut input: TRead,
    mut output: TWrite,
    inflated_size: usize,
    chksum: &mut TChecksum,
) -> Result<usize, DeflateError> {
    let mut inbuf: [u8; ENCODER_BUF_SIZE] = [0; ENCODER_BUF_SIZE];
    let mut outbuf: [u8; ENCODER_BUF_SIZE] = [0; ENCODER_BUF_SIZE];
    let mut count: usize = 0;
    let mut csize: usize = 0;

    loop {
        let len = input.read_fill(&mut inbuf)?;
        count += len;
        chksum.push(&inbuf[0..len]);
        stream.avail_in = len as _;
        let action = {
            if count == inflated_size {
                Z_FINISH
            } else {
                Z_NO_FLUSH
            }
        };
        stream.next_in = inbuf.as_mut_ptr();
        loop {
            stream.avail_out = ENCODER_BUF_SIZE as _;
            stream.next_out = outbuf.as_mut_ptr();
            unsafe {
                let err = deflate(stream, action);
                if err != Z_OK {
                    return match err {
                        Z_MEM_ERROR => Err(DeflateError::Memory),
                        Z_STREAM_ERROR => Err(DeflateError::Unsupported("compression level")),
                        Z_VERSION_ERROR => Err(DeflateError::Unsupported("version")),
                        _ => Err(DeflateError::Unknown),
                    };
                }
            }
            let len = ENCODER_BUF_SIZE - stream.avail_out as usize;
            output.write_all(&outbuf[0..len])?;
            csize += len;
            if stream.avail_out == 0 {
                break;
            }
        }
        if action == Z_FINISH {
            break;
        }
    }
    Ok(csize)
}

fn do_inflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
    stream: &mut z_stream,
    mut input: TRead,
    mut output: TWrite,
    deflated_size: usize,
    chksum: &mut TChecksum,
) -> Result<(), InflateError> {
    let mut inbuf: [u8; DECODER_BUF_SIZE] = [0; DECODER_BUF_SIZE];
    let mut outbuf: [u8; DECODER_BUF_SIZE] = [0; DECODER_BUF_SIZE];
    let mut remaining = deflated_size;

    loop {
        let len = input.read_fill(&mut inbuf)?;
        remaining -= len;
        if remaining == 0 && len == 0 {
            break;
        }
        stream.avail_in = len as _;
        stream.next_in = inbuf.as_mut_ptr();
        loop {
            stream.avail_out = DECODER_BUF_SIZE as _;
            stream.next_out = outbuf.as_mut_ptr();
            unsafe {
                let err = inflate(stream, Z_NO_FLUSH);
                match err {
                    Z_MEM_ERROR => return Err(InflateError::Memory),
                    Z_DATA_ERROR => return Err(InflateError::Data),
                    Z_NEED_DICT => return Err(InflateError::Data),
                    Z_VERSION_ERROR => return Err(InflateError::Unsupported("version")),
                    _ => (),
                }
            }
            let len = DECODER_BUF_SIZE - stream.avail_out as usize;
            chksum.push(&outbuf[0..len]);
            output.write_all(&outbuf[0..len])?;
            if stream.avail_out == 0 {
                break;
            }
        }
    }
    Ok(())
}

pub struct ZlibCompressionMethod {}

impl Deflater for ZlibCompressionMethod {
    fn deflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
        input: TRead,
        output: TWrite,
        inflated_size: usize,
        chksum: &mut TChecksum,
    ) -> Result<usize, DeflateError> {
        let mut encoder = new_encoder()?;
        let res = do_deflate(&mut encoder, input, output, inflated_size, chksum);
        unsafe {
            deflateEnd(&mut encoder);
        }
        res
    }
}

impl Inflater for ZlibCompressionMethod {
    fn inflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
        input: TRead,
        output: TWrite,
        deflated_size: usize,
        chksum: &mut TChecksum,
    ) -> Result<(), InflateError> {
        let mut decoder = new_decoder()?;
        let res = do_inflate(&mut decoder, input, output, deflated_size, chksum);
        unsafe {
            inflateEnd(&mut decoder);
        }
        res
    }
}
