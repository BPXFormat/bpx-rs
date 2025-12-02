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

use bytesutil::ReadBytes;
use std::io::Read;

use crate::sd::{error::Error, value::Type, Array, Object, Result, Value};
use crate::util::traits::ReadFill;

fn read_bool<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut flag: [u8; 1] = [0; 1];

    if stream.read_fill(&mut flag)? != 1 {
        return Err(Error::Truncation(Type::Bool));
    }
    Ok(Value::Bool(flag[0] == 1))
}

fn read_uint8<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 1] = [0; 1];

    if stream.read_fill(&mut val)? != 1 {
        return Err(Error::Truncation(Type::Uint8));
    }
    Ok(Value::Uint8(val[0]))
}

fn read_int8<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 1] = [0; 1];

    if stream.read_fill(&mut val)? != 1 {
        return Err(Error::Truncation(Type::Int8));
    }
    Ok(Value::Int8(val[0] as i8))
}

fn read_uint16<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 2] = [0; 2];

    if stream.read_fill(&mut val)? != 2 {
        return Err(Error::Truncation(Type::Uint16));
    }
    Ok(Value::Uint16(u16::read_bytes_le(&val)))
}

fn read_int16<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 2] = [0; 2];

    if stream.read_fill(&mut val)? != 2 {
        return Err(Error::Truncation(Type::Int16));
    }
    Ok(Value::Int16(i16::read_bytes_le(&val)))
}

fn read_uint32<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 4] = [0; 4];

    if stream.read_fill(&mut val)? != 4 {
        return Err(Error::Truncation(Type::Uint32));
    }
    Ok(Value::Uint32(u32::read_bytes_le(&val)))
}

fn read_int32<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 4] = [0; 4];

    if stream.read_fill(&mut val)? != 4 {
        return Err(Error::Truncation(Type::Int32));
    }
    Ok(Value::Int32(i32::read_bytes_le(&val)))
}

fn read_uint64<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 8] = [0; 8];

    if stream.read_fill(&mut val)? != 8 {
        return Err(Error::Truncation(Type::Uint64));
    }
    Ok(Value::Uint64(u64::read_bytes_le(&val)))
}

fn read_int64<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 8] = [0; 8];

    if stream.read_fill(&mut val)? != 8 {
        return Err(Error::Truncation(Type::Int64));
    }
    Ok(Value::Int64(i64::read_bytes_le(&val)))
}

fn read_float<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 4] = [0; 4];

    if stream.read_fill(&mut val)? != 4 {
        return Err(Error::Truncation(Type::Float));
    }
    Ok(Value::Float(f32::read_bytes_le(&val)))
}

fn read_double<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut val: [u8; 8] = [0; 8];

    if stream.read_fill(&mut val)? != 8 {
        return Err(Error::Truncation(Type::Double));
    }
    Ok(Value::Double(f64::read_bytes_le(&val)))
}

fn read_string<TRead: Read>(stream: &mut TRead, _: &mut usize) -> Result<Value> {
    let mut curs: Vec<u8> = Vec::new();
    let mut chr: [u8; 1] = [0; 1]; //read char by char with a buffer

    if stream.read_fill(&mut chr)? != 1 {
        return Err(Error::Truncation(Type::String));
    }
    while chr[0] != 0x0 {
        curs.push(chr[0]);
        if stream.read_fill(&mut chr)? != 1 {
            return Err(Error::Truncation(Type::String));
        }
    }
    match String::from_utf8(curs) {
        Err(_) => Err(Error::Utf8),
        Ok(v) => Ok(Value::String(v)),
    }
}

fn parse_object<TRead: Read>(stream: &mut TRead, max_depth: &mut usize) -> Result<Object> {
    *max_depth -= 1;
    if *max_depth == 0 {
        return Err(Error::MaxDepthExceeded);
    }

    let mut obj = Object::new();
    let mut count = {
        let mut buf: [u8; 1] = [0; 1];
        if stream.read_fill(&mut buf)? != 1 {
            return Err(Error::Truncation(Type::Object));
        }
        buf[0]
    };

    while count > 0 {
        let mut prop: [u8; 9] = [0; 9];
        if stream.read_fill(&mut prop)? != 9 {
            return Err(Error::Truncation(Type::Object));
        }
        let hash = u64::read_bytes_le(&prop[0..8]);
        let type_code = prop[8];
        match get_value_parser(type_code) {
            Some(func) => obj.set(hash, func(stream, max_depth)?),
            None => return Err(Error::BadTypeCode(type_code)),
        }
        count -= 1;
    }
    Ok(obj)
}

fn parse_array<TRead: Read>(stream: &mut TRead, max_depth: &mut usize) -> Result<Array> {
    *max_depth -= 1;
    if *max_depth == 0 {
        return Err(Error::MaxDepthExceeded);
    }

    let mut count = {
        let mut buf: [u8; 1] = [0; 1];
        if stream.read_fill(&mut buf)? != 1 {
            return Err(Error::Truncation(Type::Array));
        }
        buf[0]
    };
    let mut arr = Array::with_capacity(count);

    while count > 0 {
        let mut type_code: [u8; 1] = [0; 1];
        if stream.read_fill(&mut type_code)? != 1 {
            return Err(Error::Truncation(Type::Array));
        }
        match get_value_parser(type_code[0]) {
            Some(func) => arr.as_mut().push(func(stream, max_depth)?),
            None => return Err(Error::BadTypeCode(type_code[0])),
        }
        count -= 1;
    }
    Ok(arr)
}

type ValueParserFunc<TRead> = fn(stream: &mut TRead, max_depth: &mut usize) -> Result<Value>;

fn get_value_parser<TRead: Read>(type_code: u8) -> Option<ValueParserFunc<TRead>> {
    match type_code {
        0x0 => Some(|_, _| Ok(Value::Null)),
        0x1 => Some(read_bool),
        0x2 => Some(read_uint8),
        0x3 => Some(read_uint16),
        0x4 => Some(read_uint32),
        0x5 => Some(read_uint64),
        0x6 => Some(read_int8),
        0x7 => Some(read_int16),
        0x8 => Some(read_int32),
        0x9 => Some(read_int64),
        0xA => Some(read_float),
        0xB => Some(read_double),
        0xC => Some(read_string),
        0xD => Some(|stream, max_depth| Ok(Value::Array(parse_array(stream, max_depth)?))),
        0xE => Some(|stream, max_depth| Ok(Value::Object(parse_object(stream, max_depth)?))),
        _ => None,
    }
}

pub fn read_structured_data<TRead: Read>(
    mut source: TRead,
    mut max_depth: usize,
) -> Result<Object> {
    parse_object(&mut source, &mut max_depth)
}
