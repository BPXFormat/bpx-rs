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

use std::io::{Read, Seek};

use crate::util::table::NamedItemTable;
use crate::{
    core::{header::Struct, Container, Handle},
    shader::{
        error::{EosContext, Error, InvalidCodeContext},
        symbol::{Symbol, SIZE_SYMBOL_STRUCTURE},
        Result, Stage, Target, Type,
    },
};

pub fn get_target_type_from_code(acode: u8, tcode: u8) -> Result<(Target, Type)> {
    let target;
    let ty;

    match acode {
        0x1 => target = Target::DX11,
        0x2 => target = Target::DX12,
        0x3 => target = Target::GL33,
        0x4 => target = Target::GL40,
        0x5 => target = Target::GL41,
        0x6 => target = Target::GL42,
        0x7 => target = Target::GL43,
        0x8 => target = Target::GL44,
        0x9 => target = Target::GL45,
        0xA => target = Target::GL46,
        0xB => target = Target::ES30,
        0xC => target = Target::ES31,
        0xD => target = Target::ES32,
        0xE => target = Target::VK10,
        0xF => target = Target::VK11,
        0x10 => target = Target::VK12,
        0x11 => target = Target::MT,
        0xFF => target = Target::Any,
        _ => {
            return Err(Error::InvalidCode {
                context: InvalidCodeContext::Target,
                code: acode,
            })
        }
    }
    if tcode == b'A' {
        //Rust refuses to parse match properly so use if/else-if blocks
        ty = Type::Assembly;
    } else if tcode == b'P' {
        ty = Type::Pipeline;
    } else {
        return Err(Error::InvalidCode {
            context: InvalidCodeContext::Type,
            code: tcode,
        });
    }
    Ok((target, ty))
}

pub fn get_stage_from_code(code: u8) -> Result<Stage> {
    match code {
        0x0 => Ok(Stage::Vertex),
        0x1 => Ok(Stage::Hull),
        0x2 => Ok(Stage::Domain),
        0x3 => Ok(Stage::Geometry),
        0x4 => Ok(Stage::Pixel),
        _ => Err(Error::InvalidCode {
            context: InvalidCodeContext::Stage,
            code,
        }),
    }
}

pub fn read_symbol_table<T: Read + Seek>(
    container: &Container<T>,
    num_symbols: u16,
    symbol_table: Handle,
) -> Result<NamedItemTable<Symbol>> {
    let sections = container.sections();
    let count = sections[symbol_table].header().size / SIZE_SYMBOL_STRUCTURE as u32;

    if count != num_symbols as u32 {
        return Err(Error::Eos(EosContext::SymbolTable));
    }
    let mut symbols = Vec::with_capacity(count as _);
    let mut data = sections.load(symbol_table)?;
    for _ in 0..count {
        let header = Symbol::read(&mut *data)?;
        symbols.push(header);
    }
    Ok(NamedItemTable::with_list(symbols))
}
