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

#![cfg(feature = "table")]

use std::io::Seek;
use bpx::core::Container;
use bpx::core::header::{SECTION_TYPE_STRING, SECTION_TYPE_TABLE};
use bpx::strings::StringSection;
use bpx::table::column::Type;
use bpx::table::core::Table;
use bpx::table::error::Error;

#[test]
fn attempt_create_read_table() {
    let mut buffer = {
        let buffer = bpx::util::new_byte_buf(256);
        let mut container = Container::create(buffer);
        let strings = StringSection::create(&mut container).handle();
        let mut table = Table::create(&mut container, "Test", strings).unwrap();
        let mut columns = table.columns_mut(&container);
        columns.create("A", Type::Uint8, 1).unwrap();
        columns.create("B", Type::Float, 1).unwrap();
        columns.create("C", Type::Varchar, 8).unwrap();
        table.save(&container).unwrap();
        let a = table.get_column_pos(&container, "A").unwrap();
        let b = table.get_column_pos(&container, "B").unwrap();
        let c = table.get_column_pos(&container, "C").unwrap();
        {
            let mut data = container.sections().open(table.handle()).unwrap();
            let mut row = table.create_row();
            let err = row.write(0, &mut *data).unwrap_err();
            assert!(matches!(err, Error::RowIndexOutOfBounds(0)));
            let err = row.read(0, &mut *data).unwrap_err();
            assert!(matches!(err, Error::RowIndexOutOfBounds(0)));
            row.cell_mut(a).set(0xFF).unwrap();
            row.cell_mut(b).set(0.42).unwrap();
            row.cell_mut(c).set("test").unwrap();
            row.append(&mut *data).unwrap();
            row.cell_mut(c).set("value").unwrap();
            row.append(&mut *data).unwrap();
            row.cell_mut(c).set("another value").unwrap();
            row.append(&mut *data).unwrap();
        }
        container.save().unwrap();
        container.into_inner()
    };
    {
        buffer.seek(std::io::SeekFrom::Start(0)).unwrap();
        let container = Container::open(buffer).unwrap();
        let strings = container.sections().find_by_type(SECTION_TYPE_STRING).unwrap();
        let section = container.sections().find_by_type(SECTION_TYPE_TABLE).unwrap();
        let table = Table::open(&container, section, strings).unwrap();
        let a = table.get_column_pos(&container, "A").unwrap();
        let b = table.get_column_pos(&container, "B").unwrap();
        let c = table.get_column_pos(&container, "C").unwrap();
        let mut data = container.sections().open(table.handle()).unwrap();
        let mut row = table.create_row();
        // Annoying stupid language FAR too explicit...
        row.read(0, &mut *data).unwrap();
        assert!(!row.is_free());
        assert_eq!(0xFF, row.cell(a).get().unwrap());
        assert!((0.42 - row.cell(b).get::<f64>().unwrap()).abs() < 0.001);
        assert_eq!(row.cell(c).get::<&str>().unwrap(), "test");
        row.read(1, &mut *data).unwrap();
        assert!(!row.is_free());
        assert_eq!(0xFF, row.cell(a).get().unwrap());
        assert!((0.42 - row.cell(b).get::<f64>().unwrap()).abs() < 0.001);
        assert_eq!(row.cell(c).get::<&str>().unwrap(), "value");
        row.read(2, &mut *data).unwrap();
        assert!(!row.is_free());
        assert_eq!(0xFF, row.cell(a).get().unwrap());
        assert!((0.42 - row.cell(b).get::<f64>().unwrap()).abs() < 0.001);
        assert_eq!(row.cell(c).get::<&str>().unwrap(), "another ");
    }
}
