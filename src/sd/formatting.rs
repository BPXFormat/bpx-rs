// Copyright (c) 2024, BlockProject 3D
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

//! Provides formatting support for BPXSD Object.

use std::fmt::{Display, Formatter};

use crate::sd::{debug::Debugger, Array, Object, Value};

/// Type of indentation.
#[derive(Debug, Copy, Clone)]
pub enum IndentType {
    /// Indent with tabs.
    Tabs,

    /// Indent with spaces.
    Spaces,
}

struct FormatImpl {
    indent_type: IndentType,
    indent_size: usize,
    initial_indent_size: usize,
}

impl FormatImpl {
    fn fmt_value(&self, val: &Value, f: &mut Formatter) -> std::fmt::Result {
        match val {
            Value::Null => write!(f, "null"),
            Value::Bool(v) => {
                if *v {
                    f.write_str("true")
                } else {
                    f.write_str("false")
                }
            }
            Value::Uint8(v) => write!(f, "{}u8", v),
            Value::Uint16(v) => write!(f, "{}u16", v),
            Value::Uint32(v) => write!(f, "{}u32", v),
            Value::Uint64(v) => write!(f, "{}u64", v),
            Value::Int8(v) => write!(f, "{}i8", v),
            Value::Int16(v) => write!(f, "{}i16", v),
            Value::Int32(v) => write!(f, "{}i32", v),
            Value::Int64(v) => write!(f, "{}i64", v),
            Value::Float(v) => write!(f, "{}f32", v),
            Value::Double(v) => write!(f, "{}f64", v),
            Value::String(v) => write!(f, "'{}'", v),
            Value::Array(v) => {
                let ff = FormatImpl {
                    indent_type: self.indent_type,
                    indent_size: self.indent_size + self.initial_indent_size,
                    initial_indent_size: self.initial_indent_size,
                };
                ff.fmt_array(v, f)
            }
            Value::Object(v) => {
                let ff = FormatImpl {
                    indent_type: self.indent_type,
                    indent_size: self.indent_size + self.initial_indent_size,
                    initial_indent_size: self.initial_indent_size,
                };
                ff.fmt_object(v, f)
            }
        }
    }

    fn fmt_indent(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.indent_size == 0 {
            return Ok(());
        }
        for _ in 0..self.indent_size {
            match self.indent_type {
                IndentType::Tabs => f.write_str("\t"),
                IndentType::Spaces => f.write_str(" "),
            }?;
        }
        Ok(())
    }

    fn fmt_indent_half(&self, f: &mut Formatter) -> std::fmt::Result {
        let size = self.indent_size - self.initial_indent_size;
        if size == 0 {
            return Ok(());
        }
        for _ in 0..size {
            match self.indent_type {
                IndentType::Tabs => f.write_str("\t"),
                IndentType::Spaces => f.write_str(" "),
            }?;
        }
        Ok(())
    }

    fn fmt_array(&self, array: &Array, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "[")?;
        for v in array {
            self.fmt_indent(f)?;
            self.fmt_value(v, f)?;
            writeln!(f)?;
        }
        self.fmt_indent_half(f)?;
        write!(f, "]")
    }

    fn fmt_object(&self, object: &Object, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{{")?;
        if let Ok(v) = Debugger::attach(object) {
            for (name, hash, val) in &v {
                self.fmt_indent(f)?;
                if let Some(name) = name {
                    write!(f, "{}: ", name)?;
                } else {
                    write!(f, "{:#X}: ", hash.into_inner())?;
                }
                self.fmt_value(val, f)?;
                writeln!(f)?;
            }
        } else {
            for (hash, val) in object {
                self.fmt_indent(f)?;
                write!(f, "{:#X}: ", hash.into_inner())?;
                self.fmt_value(val, f)?;
                writeln!(f)?;
            }
        }
        self.fmt_indent_half(f)?;
        write!(f, "}}")
    }
}

/// Formatting context.
pub struct FormatContext<'a> {
    indent_type: IndentType,
    indent_size: usize,
    object: &'a Object,
}

impl Display for FormatContext<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let ff = FormatImpl {
            indent_type: self.indent_type,
            indent_size: self.indent_size,
            initial_indent_size: self.indent_size,
        };
        ff.fmt_object(self.object, f)
    }
}

/// Provides formatting support.
pub trait Format {
    /// Returns a formatting context.
    ///
    /// # Arguments
    ///
    /// * `indent_type`: the type of indentation.
    /// * `indent_size`: the indentation size.
    ///
    /// returns: FormatContext
    fn format(&self, indent_type: IndentType, indent_size: usize) -> FormatContext<'_>;
}

impl Format for Object {
    fn format(&self, indent_type: IndentType, indent_size: usize) -> FormatContext<'_> {
        FormatContext {
            indent_size,
            indent_type,
            object: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Format;
    use crate::sd::{debug::Debugger, formatting::IndentType, Object, Value};

    static BASIC_TABS_1: &str = "{
\tTest1: 42i32
\tTest2: 'a string'
}";

    static BASIC_TABS_2: &str = "{
\tTest2: 'a string'
\tTest1: 42i32
}";

    static BASIC_SPACES_1: &str = "{
  Test1: 42i32
  Test2: 'a string'
}";

    static BASIC_SPACES_2: &str = "{
  Test2: 'a string'
  Test1: 42i32
}";

    static ARRAY: &str = "{
  Test2: [
    'a string'
    42u32
  ]
}";

    static OBJECT_1: &str = "{
  Test: {
    Test2: 'a string'
    Test1: 42i32
  }
}";

    static OBJECT_2: &str = "{
  Test: {
    Test1: 42i32
    Test2: 'a string'
  }
}";

    static OBJECT_IN_OBJECT_1: &str = "{
  Test2: 42i32
  Test1: {
    Test2: 'a string'
  }
}";

    static OBJECT_IN_OBJECT_2: &str = "{
  Test1: {
    Test2: 'a string'
  }
  Test2: 42i32
}";

    fn basic_object() -> Object {
        let mut obj = Debugger::attach(Object::new()).unwrap();
        obj.set("Test1", 42.into());
        obj.set("Test2", "a string".into());
        obj.detach()
    }

    #[test]
    fn basic_spaces() {
        let str = basic_object().format(IndentType::Spaces, 2).to_string();
        assert!(str == BASIC_SPACES_1 || str == BASIC_SPACES_2);
    }

    #[test]
    fn basic_tabs() {
        let str = basic_object().format(IndentType::Tabs, 1).to_string();
        assert!(str == BASIC_TABS_1 || str == BASIC_TABS_2);
    }

    #[test]
    fn array() {
        let mut obj = Debugger::attach(Object::new()).unwrap();
        obj.set(
            "Test2",
            vec![Value::from("a string"), Value::from(42u32)].into(),
        );
        let str = obj.detach().format(IndentType::Spaces, 2).to_string();
        assert_eq!(str, ARRAY);
    }

    #[test]
    fn object() {
        let mut obj = Debugger::attach(Object::new()).unwrap();
        obj.set("Test", basic_object().into());
        let str = obj.detach().format(IndentType::Spaces, 2).to_string();
        assert!(str == OBJECT_1 || str == OBJECT_2);
    }

    #[test]
    fn object_in_object() {
        let mut sub = Debugger::attach(Object::new()).unwrap();
        sub.set("Test2", "a string".into());
        let mut obj = Debugger::attach(Object::new()).unwrap();
        obj.set("Test2", 42.into());
        obj.set("Test1", sub.detach().into());
        let str = obj.detach().format(IndentType::Spaces, 2).to_string();
        assert!(str == OBJECT_IN_OBJECT_1 || str == OBJECT_IN_OBJECT_2);
    }
}
