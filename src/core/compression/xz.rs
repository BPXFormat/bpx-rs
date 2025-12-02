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

use lzma_sys::{
    lzma_code, lzma_easy_encoder, lzma_end, lzma_mt, lzma_stream, lzma_stream_decoder,
    lzma_stream_encoder_mt, LZMA_BUF_ERROR, LZMA_CHECK_NONE, LZMA_CONCATENATED, LZMA_DATA_ERROR,
    LZMA_FINISH, LZMA_MEM_ERROR, LZMA_OK, LZMA_OPTIONS_ERROR, LZMA_PRESET_EXTREME, LZMA_RUN,
    LZMA_STREAM_END, LZMA_UNSUPPORTED_CHECK,
};

use crate::core::{
    compression::{Checksum, Deflater, Inflater},
    error::{DeflateError, InflateError},
};
use crate::util::traits::ReadFill;

const THREADS_MAX: u32 = 8;
const ENCODER_BUF_SIZE: usize = 8192;
const DECODER_BUF_SIZE: usize = ENCODER_BUF_SIZE * 2;

fn new_encoder() -> Result<lzma_stream, DeflateError> {
    unsafe {
        let mut stream: lzma_stream = std::mem::zeroed();
        let mut mt: lzma_mt = std::mem::zeroed();

        mt.flags = 0;
        mt.block_size = 0;
        mt.timeout = 0;
        mt.preset = LZMA_PRESET_EXTREME;
        mt.filters = std::ptr::null();
        mt.check = LZMA_CHECK_NONE;
        mt.threads = num_cpus::get() as u32;
        let res;
        if mt.threads == 0 || mt.threads == 1 {
            res = lzma_easy_encoder(&mut stream, LZMA_PRESET_EXTREME, LZMA_CHECK_NONE);
        } else {
            if mt.threads > THREADS_MAX {
                mt.threads = THREADS_MAX;
            }
            res = lzma_stream_encoder_mt(&mut stream, &mt);
        }
        if res == LZMA_OK {
            return Ok(stream);
        }
        match res {
            LZMA_MEM_ERROR => Err(DeflateError::Memory),
            LZMA_OPTIONS_ERROR => Err(DeflateError::Unsupported("filter chain")),
            LZMA_UNSUPPORTED_CHECK => Err(DeflateError::Unsupported("integrity check")),
            _ => Err(DeflateError::Unknown),
        }
    }
}

fn new_decoder() -> Result<lzma_stream, InflateError> {
    unsafe {
        let mut stream: lzma_stream = std::mem::zeroed();
        let res = lzma_stream_decoder(&mut stream, u32::MAX as u64, LZMA_CONCATENATED);
        if res == LZMA_OK {
            return Ok(stream);
        }
        match res {
            LZMA_MEM_ERROR => Err(InflateError::Memory),
            LZMA_OPTIONS_ERROR => Err(InflateError::Unsupported("filter chain")),
            LZMA_UNSUPPORTED_CHECK => Err(InflateError::Unsupported("integrity check")),
            _ => Err(InflateError::Unknown),
        }
    }
}

fn do_deflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
    stream: &mut lzma_stream,
    mut input: TRead,
    mut output: TWrite,
    inflated_size: usize,
    chksum: &mut TChecksum,
) -> Result<usize, DeflateError> {
    let mut action = LZMA_RUN;
    let mut inbuf: [u8; ENCODER_BUF_SIZE] = [0; ENCODER_BUF_SIZE];
    let mut outbuf: [u8; ENCODER_BUF_SIZE] = [0; ENCODER_BUF_SIZE];
    let mut count: usize = 0;
    let mut csize: usize = 0;

    stream.next_in = inbuf.as_ptr();
    stream.avail_in = 0;
    stream.next_out = outbuf.as_mut_ptr();
    stream.avail_out = ENCODER_BUF_SIZE;
    loop {
        if stream.avail_in == 0 && count < inflated_size {
            let len = input.read_fill(&mut inbuf)?;
            count += len;
            chksum.push(&inbuf[0..len]);
            stream.avail_in = len;
            stream.next_in = inbuf.as_ptr();
            if count == inflated_size {
                action = LZMA_FINISH;
            }
        }
        unsafe {
            let res = lzma_code(stream, action);
            if stream.avail_out == 0 || res == LZMA_STREAM_END {
                let size = ENCODER_BUF_SIZE - stream.avail_out;
                csize += size;
                output.write_all(&outbuf[0..size])?;
                stream.avail_out = ENCODER_BUF_SIZE;
                stream.next_out = outbuf.as_mut_ptr();
            }
            if res != LZMA_OK {
                if res == LZMA_STREAM_END {
                    break;
                }
                return match res {
                    LZMA_MEM_ERROR => Err(DeflateError::Memory),
                    LZMA_DATA_ERROR => Err(DeflateError::Data),
                    _ => Err(DeflateError::Unknown),
                };
            }
        }
    }
    Ok(csize)
}

fn do_inflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
    stream: &mut lzma_stream,
    mut input: TRead,
    mut output: TWrite,
    deflated_size: usize,
    chksum: &mut TChecksum,
) -> Result<(), InflateError> {
    let mut action = LZMA_RUN;
    let mut inbuf: [u8; ENCODER_BUF_SIZE] = [0; ENCODER_BUF_SIZE];
    let mut outbuf: [u8; DECODER_BUF_SIZE] = [0; DECODER_BUF_SIZE];
    let mut remaining = deflated_size;

    stream.next_in = inbuf.as_ptr();
    stream.avail_in = 0;
    stream.next_out = outbuf.as_mut_ptr();
    stream.avail_out = DECODER_BUF_SIZE;
    loop {
        if stream.avail_in == 0 && remaining > 0 {
            let res = input.read_fill(&mut inbuf[0..std::cmp::min(ENCODER_BUF_SIZE, remaining)])?;
            remaining -= res;
            stream.avail_in = res;
            stream.next_in = inbuf.as_ptr();
            if remaining == 0 {
                action = LZMA_FINISH;
            }
        }
        unsafe {
            let res = lzma_code(stream, action);
            if stream.avail_out == 0 || res == LZMA_STREAM_END {
                let size = DECODER_BUF_SIZE - stream.avail_out;
                chksum.push(&outbuf[0..size]);
                output.write_all(&outbuf[0..size])?;
                stream.avail_out = DECODER_BUF_SIZE;
                stream.next_out = outbuf.as_mut_ptr();
            }
            if res != LZMA_OK {
                if res == LZMA_STREAM_END {
                    break;
                }
                return match res {
                    LZMA_MEM_ERROR => Err(InflateError::Memory),
                    LZMA_DATA_ERROR | LZMA_BUF_ERROR => Err(InflateError::Data),
                    _ => Err(InflateError::Unknown),
                };
            }
        }
    }
    Ok(())
}

pub struct XzCompressionMethod {}

impl Deflater for XzCompressionMethod {
    fn deflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
        input: TRead,
        output: TWrite,
        inflated_size: usize,
        chksum: &mut TChecksum,
    ) -> Result<usize, DeflateError> {
        let mut stream = new_encoder()?;
        let res = do_deflate(&mut stream, input, output, inflated_size, chksum);
        unsafe {
            lzma_end(&mut stream);
        }
        res
    }
}

impl Inflater for XzCompressionMethod {
    fn inflate<TRead: Read, TWrite: Write, TChecksum: Checksum>(
        input: TRead,
        output: TWrite,
        deflated_size: usize,
        chksum: &mut TChecksum,
    ) -> Result<(), InflateError> {
        let mut stream = new_decoder()?;
        let res = do_inflate(&mut stream, input, output, deflated_size, chksum);
        unsafe {
            lzma_end(&mut stream);
        }
        res
    }
}
