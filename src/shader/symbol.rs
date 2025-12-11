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

//! Contains utilities to work with the symbol table section.

use bytesutil::{ReadBytes, WriteBytes};

use crate::util::table::Item;
use crate::{
    core::header::Struct,
    sd::Value,
    shader::{
        error::{EosContext, Error, InvalidCodeContext},
        Stage,
    },
};

/// Indicates this symbol is used on the vertex stage.
pub const FLAG_VERTEX_STAGE: u16 = 0x1;

/// Indicates this symbol is used on the hull stage.
pub const FLAG_HULL_STAGE: u16 = 0x2;

/// Indicates this symbol is used on the domain stage.
pub const FLAG_DOMAIN_STAGE: u16 = 0x4;

/// Indicates this symbol is used on the geometry stage.
pub const FLAG_GEOMETRY_STAGE: u16 = 0x8;

/// Indicates this symbol is used on the pixel stage.
pub const FLAG_PIXEL_STAGE: u16 = 0x10;

/// Indicates this symbol is part of an assembly.
pub const FLAG_ASSEMBLY: u16 = 0x20;

/// Indicates this symbol is not defined in this package.
pub const FLAG_EXTERNAL: u16 = 0x40;

/// Indicates this symbol is defined in this package.
pub const FLAG_INTERNAL: u16 = 0x80;

/// Indicates this symbol has extended data.
pub const FLAG_EXTENDED_DATA: u16 = 0x100;

/// Indicates this symbol has a register number.
pub const FLAG_REGISTER: u16 = 0x200;

/// Size in bytes of a symbol structure.
pub const SIZE_SYMBOL_STRUCTURE: usize = 12;

/// The type of a symbol.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    /// A texture symbol.
    Texture,

    /// A sampler symbol.
    Sampler,

    /// A constant buffer symbol.
    ConstantBuffer,

    /// A high performance constant symbol (represented as push constants in vulkan).
    Constant,

    /// A vertex format symbol.
    VertexFormat,

    /// A pipeline symbol.
    Pipeline,

    /// A render target output symbol.
    Output,
}

fn get_symbol_type_from_code(scode: u8) -> Result<Type, Error> {
    match scode {
        0x0 => Ok(Type::Texture),
        0x1 => Ok(Type::Sampler),
        0x2 => Ok(Type::ConstantBuffer),
        0x3 => Ok(Type::Constant),
        0x4 => Ok(Type::VertexFormat),
        0x5 => Ok(Type::Pipeline),
        0x6 => Ok(Type::Output),
        _ => Err(Error::InvalidCode {
            context: InvalidCodeContext::SymbolType,
            code: scode,
        }),
    }
}

/// Represents the structure of a symbol.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Symbol {
    /// The pointer to the name of the symbol.
    pub name: u32,

    /// The pointer to the BPXSD object attached to this symbol.
    pub extended_data: u32,

    /// The symbol flags (see the FLAG_ constants in the [symbol](crate::shader::symbol) module).
    pub flags: u16,

    /// The type of symbol.
    pub ty: Type,

    /// The register number for this symbol.
    pub register: u8,
}

impl Struct<SIZE_SYMBOL_STRUCTURE> for Symbol {
    type Output = Symbol;
    type Error = Error;

    fn new() -> Self {
        Symbol {
            name: 0,
            extended_data: 0xFFFFFF,
            flags: 0,
            ty: Type::Constant,
            register: 0xFF,
        }
    }

    fn error_buffer_size() -> Option<Self::Error> {
        Some(Error::Eos(EosContext::SymbolTable))
    }

    fn from_bytes(buffer: [u8; SIZE_SYMBOL_STRUCTURE]) -> Result<Self::Output, Self::Error> {
        let name = u32::read_bytes_le(&buffer[0..4]);
        let extended_data = u32::read_bytes_le(&buffer[4..8]);
        let flags = u16::read_bytes_le(&buffer[8..10]);
        let ty = get_symbol_type_from_code(buffer[10])?;
        let register = buffer[11];
        Ok(Symbol {
            name,
            extended_data,
            flags,
            ty,
            register,
        })
    }

    fn to_bytes(&self) -> [u8; SIZE_SYMBOL_STRUCTURE] {
        let mut buf = [0; SIZE_SYMBOL_STRUCTURE];
        self.name.write_bytes_le(&mut buf[0..4]);
        self.extended_data.write_bytes_le(&mut buf[4..8]);
        self.flags.write_bytes_le(&mut buf[8..10]);
        match self.ty {
            Type::Texture => buf[10] = 0x0,
            Type::Sampler => buf[10] = 0x1,
            Type::ConstantBuffer => buf[10] = 0x2,
            Type::Constant => buf[10] = 0x3,
            Type::VertexFormat => buf[10] = 0x4,
            Type::Pipeline => buf[10] = 0x5,
            Type::Output => buf[10] = 0x6,
        };
        buf[11] = self.register;
        buf
    }
}

impl Item for Symbol {
    fn get_name_address(&self) -> u32 {
        self.name
    }
}

/// The required settings to create a new symbol.
///
/// *This is intended to be generated with help of [Options].*
#[derive(Clone)]
pub struct Settings {
    /// The name of the symbol.
    pub name: String,

    /// The extended data [Value] of the symbol.
    pub extended_data: Value,

    /// The symbol type.
    pub ty: Type,

    /// The symbol flags.
    pub flags: u16,

    /// The symbol register number.
    pub register: u8,
}

/// Utility to simplify generation of [Settings] required when creating a new BPXS.
pub struct Options {
    sym: Settings,
}

impl Options {
    /// Creates a new set of options for a BPXS symbol.
    pub fn new<S: Into<String>>(name: S) -> Options {
        Options {
            sym: Settings {
                name: name.into(),
                extended_data: Value::Null,
                ty: Type::Constant,
                flags: 0,
                register: 0xFF,
            },
        }
    }

    /// Defines the type of this symbol.
    ///
    /// # Arguments
    ///
    /// * `ty`: the symbol type.
    pub fn ty(&mut self, ty: Type) -> &mut Self {
        self.sym.ty = ty;
        self
    }

    /// Defines the extended data for this symbol.
    ///
    /// *This function automatically adds the [FLAG_EXTENDED_DATA] flag.*
    ///
    /// # Arguments
    ///
    /// * `val`: A [Value] to store as extended data.
    pub fn extended_data(&mut self, val: Value) -> &mut Self {
        self.sym.extended_data = val;
        self.sym.flags |= FLAG_EXTENDED_DATA;
        self
    }

    /// Defines the register number of this symbol.
    ///
    /// *This function automatically adds the [FLAG_REGISTER] flag.*
    ///
    /// # Arguments
    ///
    /// * `register`: the register number of this symbol.
    pub fn register(&mut self, register: u8) -> &mut Self {
        self.sym.register = register;
        self.sym.flags |= FLAG_REGISTER;
        self
    }

    /// Marks this symbol as internal.
    ///
    /// *Adds the [FLAG_INTERNAL].*
    pub fn internal(&mut self) -> &mut Self {
        self.sym.flags |= FLAG_INTERNAL;
        self
    }

    /// Marks this symbol as external.
    ///
    /// *Adds the [FLAG_EXTERNAL].*
    pub fn external(&mut self) -> &mut Self {
        self.sym.flags |= FLAG_EXTERNAL;
        self
    }

    /// Marks this symbol as being part of an assembly.
    ///
    /// *Adds the [FLAG_ASSEMBLY].*
    pub fn assembly(&mut self) -> &mut Self {
        self.sym.flags |= FLAG_ASSEMBLY;
        self
    }

    /// Adds the stage flag identified by `stage` to this symbol.
    ///
    /// For more information please have a look at the FLAG_*_STAGE flags defined in the
    /// [symbol](crate::shader::symbol) module.
    ///
    /// # Arguments
    ///
    /// * `stage`: the stage to add.
    pub fn stage(&mut self, stage: Stage) -> &mut Self {
        match stage {
            Stage::Vertex => self.sym.flags |= FLAG_VERTEX_STAGE,
            Stage::Hull => self.sym.flags |= FLAG_HULL_STAGE,
            Stage::Domain => self.sym.flags |= FLAG_DOMAIN_STAGE,
            Stage::Geometry => self.sym.flags |= FLAG_GEOMETRY_STAGE,
            Stage::Pixel => self.sym.flags |= FLAG_PIXEL_STAGE,
        }
        self
    }

    /// Returns the built settings.
    pub fn build(&self) -> Settings {
        self.sym.clone()
    }
}

impl From<&mut Options> for Settings {
    fn from(options: &mut Options) -> Self {
        options.build()
    }
}

impl From<Options> for Settings {
    fn from(options: Options) -> Self {
        options.build()
    }
}
