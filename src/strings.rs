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

//! A set of helpers to manipulate BPX strings sections

use std::{collections::HashMap, fs::DirEntry, io::SeekFrom, path::Path, string::String};

use crate::{error::Error, section::SectionData, Interface, Result, SectionHandle};
use std::collections::hash_map::Entry;

/// Helper class to manage a BPX string section
pub struct StringSection
{
    handle: SectionHandle,
    cache: HashMap<u32, String>
}

impl StringSection
{
    /// Create a new string section from a handle
    ///
    /// # Arguments
    ///
    /// * `hdl` - handle to the string section
    ///
    /// # Returns
    ///
    /// * a new StringSection
    pub fn new(hdl: SectionHandle) -> StringSection
    {
        return StringSection {
            handle: hdl,
            cache: HashMap::new()
        };
    }

    /// Reads a string from the section
    ///
    /// # Arguments
    ///
    /// * `interface` - the BPX IO interface
    /// * `address` - the offset to the start of the string
    ///
    /// # Returns
    ///
    /// * the string read
    /// * an [Error](crate::error::Error) if the string could not be read or the section is corrupted/truncated
    pub fn get<TInterface: Interface>(&mut self, interface: &mut TInterface, address: u32) -> Result<&str>
    {
        let res = match self.cache.entry(address) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(o) => {
                let data = interface.open_section(self.handle)?;
                let s = low_level_read_string(address, data)?;
                o.insert(s)
            }
        };
        return Ok(res);
    }

    /// Writes a new string into the section
    ///
    /// # Arguments
    ///
    /// * `interface` - the BPX IO interface
    /// * `s` - the string to write
    ///
    /// # Returns
    ///
    /// * the offset to the start of the newly written string
    /// * an [Error](crate::error::Error) if the string could not be written
    pub fn put<TInterface: Interface>(&mut self, interface: &mut TInterface, s: &str) -> Result<u32>
    {
        let data = interface.open_section(self.handle)?;
        let address = low_level_write_string(s, data)?;
        self.cache.insert(address, String::from(s));
        return Ok(address);
    }
}

fn low_level_read_string(ptr: u32, string_section: &mut dyn SectionData) -> Result<String>
{
    let mut curs: Vec<u8> = Vec::new();
    let mut chr: [u8; 1] = [0; 1]; //read char by char with a buffer

    string_section.seek(SeekFrom::Start(ptr as u64))?;
    string_section.read(&mut chr)?;
    while chr[0] != 0x0 {
        curs.push(chr[0]);
        let res = string_section.read(&mut chr)?;
        if res != 1 {
            return Err(Error::Truncation("string secton read"));
        }
    }
    return match String::from_utf8(curs) {
        Err(_) => Err(Error::Utf8("string section read")),
        Ok(v) => Ok(v)
    }
}

fn low_level_write_string(s: &str, string_section: &mut dyn SectionData) -> Result<u32>
{
    let ptr = string_section.size() as u32;
    string_section.write(s.as_bytes())?;
    string_section.write(&[0x0])?;
    return Ok(ptr);
}

/// Returns the file name as a UTF-8 string from a rust Path or panics if the path is not unicode compatible (BPX only supports UTF-8)
///
/// # Arguments
///
/// * `path` - the rust Path
///
/// # Returns
///
/// * the file name as UTF-8 string
/// * an [Error](crate::error::Error) if the given Path does not have a file name
pub fn get_name_from_path(path: &Path) -> Result<String>
{
    match path.file_name() {
        Some(v) => match v.to_str() {
            Some(v) => return Ok(String::from(v)),
            // Panic here as a non Unicode system in all cases could just throw a bunch of broken unicode strings in a BPXP
            // The reason BPXP cannot support non-unicode strings in paths is simply because this would be incompatible with unicode systems
            None => panic!("Non unicode paths operating systems cannot run BPXP")
        },
        None => return Err(Error::from("incorrect path format"))
    }
}

/// Returns the file name as a UTF-8 string from a rust DirEntry or panics if the path is not unicode compatible (BPX only supports UTF-8)
///
/// # Arguments
///
/// * `entry` - the rust DirEntry
///
/// # Returns
///
/// * the file name as UTF-8 string
pub fn get_name_from_dir_entry(entry: &DirEntry) -> String
{
    match entry.file_name().to_str() {
        Some(v) => return String::from(v),
        None => panic!("Non unicode paths operating systems cannot run BPXP")
    }
}
