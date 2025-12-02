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

use std::{
    convert::{From, TryFrom, TryInto},
    string::String,
};

use crate::sd::{error::TypeError, Array, Object};
use crate::util::macros::{impl_err_conversion, named_enum};

named_enum!(
    /// Represents the value type.
    #[derive(Copy, Clone)]
    Type {
        /// A null type.
        Null: "null",

        /// A bool type.
        Bool: "bool",

        /// An 8 bit unsigned integer type.
        Uint8: "uint8",

        /// A 16 bit unsigned integer type.
        Uint16: "uint16",

        /// A 32 bit unsigned integer type.
        Uint32: "uint32",

        /// An 64 bit unsigned integer type.
        Uint64: "uint64",

        /// An 8 bit integer type.
        Int8: "int8",

        /// A 16 bit integer type.
        Int16: "int16",

        /// A 32 bit integer type.
        Int32: "int32",

        /// A 64 bit integer type.
        Int64: "int64",

        /// A 32 bit float type.
        Float: "float",

        /// A 64 bit float type.
        Double: "double",

        /// A string type.
        String: "string",

        /// An array type.
        Array: "array",

        /// An object type.
        Object: "object"
    }
);

/// Represents a BPXSD value.
#[derive(PartialEq, Clone)]
pub enum Value {
    /// NULL (0x0)
    Null,

    /// bool (0x1)
    Bool(bool),

    /// u8 (0x2)
    Uint8(u8),

    /// u16 (0x3)
    Uint16(u16),

    /// u32 (0x4)
    Uint32(u32),

    /// u64 (0x5)
    Uint64(u64),

    /// i8 (0x6)
    Int8(i8),

    /// i16 (0x7)
    Int16(i16),

    /// i32 (0x8)
    Int32(i32),

    /// i64 (0x9)
    Int64(i64),

    /// f32 (0xA)
    Float(f32),

    /// f64 (0xB)
    Double(f64),

    /// [String](String) (0xC)
    String(String),

    /// [Array](Array) (0xD)
    Array(Array),

    /// [Object](Object) (0xE)
    Object(Object),
}

impl Value {
    /// Gets the type of this Value.
    pub fn get_type(&self) -> Type {
        match self {
            Value::Null => Type::Null,
            Value::Bool(_) => Type::Bool,
            Value::Uint8(_) => Type::Uint8,
            Value::Uint16(_) => Type::Uint16,
            Value::Uint32(_) => Type::Uint32,
            Value::Uint64(_) => Type::Uint64,
            Value::Int8(_) => Type::Int8,
            Value::Int16(_) => Type::Int16,
            Value::Int32(_) => Type::Int32,
            Value::Int64(_) => Type::Int64,
            Value::Float(_) => Type::Float,
            Value::Double(_) => Type::Double,
            Value::String(_) => Type::String,
            Value::Array(_) => Type::Array,
            Value::Object(_) => Type::Object,
        }
    }

    /// Checks if this value is null.
    pub fn is_null(&self) -> bool {
        self == &Value::Null
    }

    /// Returns this value, replacing self with Null.
    pub fn take(&mut self) -> Value {
        std::mem::replace(self, Value::Null)
    }

    /// Attempts to write the object to the given IO backend.
    ///
    /// # Arguments
    ///
    /// * `dest`: the destination [Write](std::io::Write).
    /// * `max_depth`: maximum depth for nested BPXSD values.
    ///
    /// returns: Result<(), Error>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    /// use bpx::sd::Value;
    ///
    /// let mut obj = Object::new();
    /// obj.set("Test", 12.into());
    /// let mut buf = Vec::<u8>::new();
    /// Value::from(obj).write(&mut buf, 4).unwrap();
    /// assert!(buf.len() > 0);
    /// ```
    pub fn write<TWrite: std::io::Write>(
        &self,
        dest: TWrite,
        max_depth: usize,
    ) -> super::Result<()> {
        match self.as_object() {
            Some(v) => super::encoder::write_structured_data(dest, v, max_depth),
            None => Err(super::error::Error::NotAnObject),
        }
    }

    /// Attempts to read a BPXSD object from an IO backend.
    ///
    /// # Arguments
    ///
    /// * `source`: the source [Read](std::io::Read).
    /// * `max_depth`: maximum depth for nested BPXSD values.
    ///
    /// returns: Result<Object, Error>
    ///
    /// # Examples
    ///
    /// ```
    /// use bpx::sd::Object;
    /// use bpx::sd::Value;
    ///
    /// let mut obj = Object::new();
    /// obj.set("Test", 12.into());
    /// let mut buf = Vec::<u8>::new();
    /// Value::from(obj).write(&mut buf, 4).unwrap();
    /// let obj1 = Value::read(&mut buf.as_slice(), 4).unwrap();
    /// assert!(obj1.as_object().unwrap().get("Test").is_some());
    /// assert!(obj1.as_object().unwrap().get("Test").unwrap() == &Value::from(12));
    /// ```
    pub fn read<TRead: std::io::Read>(source: TRead, max_depth: usize) -> super::Result<Value> {
        super::decoder::read_structured_data(source, max_depth).map(|v| v.into())
    }
}

macro_rules! auto_as_scalar {
    ($(($func: ident $out: ty) => ($($variant: ident)*)),*) => {
        impl Value {
            $(
                /// Converts this BPXSD value to a rust type if it is compatible.
                ///
                /// Returns None if the value is not convertible to the specified type.
                /// Note that unlike try_into this function will allow similar compatible type
                /// conversions whereas try_into will always expect that the type *exactly*
                /// matches.
                pub fn $func (&self) -> Option<$out> {
                    match self {
                        $(Value::$variant(v) => Some(*v as $out),)*
                        _ => None
                    }
                }
            )*
        }
    };
}

auto_as_scalar! {
    (as_u8 u8) => (Uint8),
    (as_u16 u16) => (Uint8 Uint16),
    (as_u32 u32) => (Uint8 Uint16 Uint32),
    (as_u64 u64) => (Uint8 Uint16 Uint32 Uint64),
    (as_i8 i8) => (Int8),
    (as_i16 i16) => (Int8 Int16),
    (as_i32 i32) => (Int8 Int16 Int32),
    (as_i64 i64) => (Int8 Int16 Int32 Int64),
    (as_f32 f32) => (Float),
    (as_f64 f64) => (Float Double),
    (as_bool bool) => (Bool)
}

impl Value {
    /// Converts this BPXSD value to a string.
    ///
    /// Returns None if this value is not a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(v) => Some(v),
            _ => None,
        }
    }

    /// Converts this BPXSD value to an array.
    ///
    /// Returns None if this value is not an array.
    pub fn as_array(&self) -> Option<&Array> {
        match self {
            Value::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Converts this BPXSD value to an object.
    ///
    /// Returns None if this value is not an object.
    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(v) => Some(v),
            _ => None,
        }
    }
}

impl_err_conversion!(
    Value {
        bool => Bool,
        u8 => Uint8,
        u16 => Uint16,
        u32 => Uint32,
        u64 => Uint64,
        i8 => Int8,
        i16 => Int16,
        i32 => Int32,
        i64 => Int64,
        f32 => Float,
        f64 => Double,
        String => String,
        Array => Array,
        Object => Object
    }
);

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(String::from(v))
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        if let Some(v) = v {
            return v.into();
        }
        Value::Null
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        let mut arr = Array::new();
        for v1 in v {
            arr.as_mut().push(v1.into());
        }
        Value::Array(arr)
    }
}

macro_rules! impl_try_into_scalar {
    ($(($t: ident $out: ty)),*) => {
        $(
            impl TryFrom<Value> for $out {
                type Error = TypeError;

                fn try_from(v: Value) -> Result<Self, TypeError> {
                    match v {
                        Value::$t(v) => Ok(v),
                        _ => Err(TypeError::new(Type::$t, v.get_type()))
                    }
                }
            }

            impl TryFrom<&Value> for $out {
                type Error = TypeError;

                fn try_from(v: &Value) -> Result<Self, TypeError> {
                    match v {
                        Value::$t(v) => Ok(*v),
                        _ => Err(TypeError::new(Type::$t, v.get_type()))
                    }
                }
            }
        )*
    };
}

impl_try_into_scalar! {
    (Bool bool),
    (Uint8 u8),
    (Uint16 u16),
    (Uint32 u32),
    (Uint64 u64),
    (Int8 i8),
    (Int16 i16),
    (Int32 i32),
    (Int64 i64),
    (Float f32),
    (Double f64)
}

impl TryFrom<Value> for String {
    type Error = TypeError;

    fn try_from(v: Value) -> Result<Self, TypeError> {
        if let Value::String(v) = v {
            return Ok(v);
        }
        Err(TypeError::new(Type::String, v.get_type()))
    }
}

impl TryFrom<Value> for Array {
    type Error = TypeError;

    fn try_from(v: Value) -> Result<Self, TypeError> {
        if let Value::Array(v) = v {
            return Ok(v);
        }
        Err(TypeError::new(Type::Array, v.get_type()))
    }
}

impl TryFrom<Value> for Object {
    type Error = TypeError;

    fn try_from(v: Value) -> Result<Self, TypeError> {
        if let Value::Object(v) = v {
            return Ok(v);
        }
        Err(TypeError::new(Type::Object, v.get_type()))
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = TypeError;

    fn try_from(v: &'a Value) -> Result<Self, TypeError> {
        if let Value::String(v) = v {
            return Ok(v);
        }
        Err(TypeError::new(Type::String, v.get_type()))
    }
}

impl<'a> TryFrom<&'a Value> for &'a Array {
    type Error = TypeError;

    fn try_from(v: &'a Value) -> Result<Self, TypeError> {
        if let Value::Array(v) = v {
            return Ok(v);
        }
        Err(TypeError::new(Type::Array, v.get_type()))
    }
}

impl<'a> TryFrom<&'a Value> for &'a Object {
    type Error = TypeError;

    fn try_from(v: &'a Value) -> Result<Self, TypeError> {
        if let Value::Object(v) = v {
            return Ok(v);
        }
        Err(TypeError::new(Type::Object, v.get_type()))
    }
}

macro_rules! generate_option_try_from {
    ($($t:ident)*) => {
        $(
            impl TryFrom<Value> for Option<$t>
            {
                type Error = TypeError;

                fn try_from(v: Value) -> Result<Self, TypeError>
                {
                    if v.is_null()
                    {
                        return Ok(None);
                    }
                    let v = v.try_into()?;
                    return Ok(Some(v));
                }
            }
        )*
    };
}

macro_rules! generate_option_try_from_ref {
    ($($t:ident)*) => {
        $(
            impl<'a> TryFrom<&'a Value> for Option<&'a $t>
            {
                type Error = TypeError;

                fn try_from(v: &'a Value) -> Result<Self, TypeError>
                {
                    if v.is_null()
                    {
                        return Ok(None);
                    }
                    let v = v.try_into()?;
                    Ok(Some(v))
                }
            }
        )*
    };
}

macro_rules! generate_option_try_from_ref_scalar {
    ($($t:ident)*) => {
        $(
            impl<'a> TryFrom<&'a Value> for Option<$t>
            {
                type Error = TypeError;

                fn try_from(v: &'a Value) -> Result<Self, TypeError>
                {
                    if v.is_null()
                    {
                        return Ok(None);
                    }
                    let v = v.try_into()?;
                    Ok(Some(v))
                }
            }
        )*
    };
}

generate_option_try_from! {
    u8 u16 u32 u64
    i8 i16 i32 i64
    f32 f64 bool
    String Array Object
}

generate_option_try_from_ref! {
    Array Object str
}

generate_option_try_from_ref_scalar! {
    u8 u16 u32 u64
    i8 i16 i32 i64
    f32 f64 bool
}
