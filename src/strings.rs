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

//! A set of helpers to manipulate BPX string sections.

use std::{collections::HashMap, fs::DirEntry, io::SeekFrom, path::Path, string::String};

use crate::{error::Error, section::SectionData, Interface, Result, SectionHandle};
use std::collections::hash_map::Entry;

/// Helper class to manage a BPX string section.
///
/// # Examples
///
/// ```
/// use bpx::encoder::Encoder;
/// use bpx::header::SectionHeader;
/// use bpx::strings::StringSection;
///
/// let mut file = Encoder::new(Vec::<u8>::new()).unwrap();
/// let handle = file.create_section(SectionHeader::new()).unwrap();
/// let mut strings = StringSection::new(handle);
/// let offset = strings.put(&mut file, "Test").unwrap();
/// let str = strings.get(&mut file, offset).unwrap();
/// assert_eq!(str, "Test");
/// ```
pub struct StringSection
{
    handle: SectionHandle,
    cache: HashMap<u32, String>
}

impl StringSection
{
    /// Create a new string section from a handle.
    ///
    /// # Arguments
    ///
    /// * `hdl`: handle to the string section.
    ///
    /// returns: StringSection
    pub fn new(hdl: SectionHandle) -> StringSection
    {
        return StringSection {
            handle: hdl,
            cache: HashMap::new()
        };
    }

    /// Reads a string from the section.
    ///
    /// # Arguments
    ///
    /// * `interface`: the BPX IO interface.
    /// * `address`: the offset to the start of the string.
    ///
    /// returns: Result<&str, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error](crate::error::Error) if the string could not be read or the
    /// section is corrupted/truncated.
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

    /// Writes a new string into the section.
    ///
    /// # Arguments
    ///
    /// * `interface`: the BPX IO interface.
    /// * `s`: the string to write.
    ///
    /// returns: Result<u32, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error](crate::error::Error) if the string could not be written.
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

/// Returns the file name as a UTF-8 string from a rust Path.
///
/// # Arguments
///
/// * `path`: the rust [Path](std::path::Path).
///
/// returns: Result<String, Error>
///
/// # Errors
///
/// Returns an [Error](crate::error::Error) if the path does not have a file name.
///
/// # Panics
///
/// Panics in case `path` is not unicode compatible (BPX only supports UTF-8).
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use bpx::strings::get_name_from_path;
///
/// let str = get_name_from_path(Path::new("test/file.txt")).unwrap();
/// assert_eq!(str, "file.txt");
/// ```
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

/// Returns the file name as a UTF-8 string from a rust DirEntry.
///
/// # Arguments
///
/// * `entry`: the rust DirEntry.
///
/// returns: String
///
/// # Panics
///
/// Panics in case `entry` is not unicode compatible (BPX only supports UTF-8).
pub fn get_name_from_dir_entry(entry: &DirEntry) -> String
{
    match entry.file_name().to_str() {
        Some(v) => return String::from(v),
        None => panic!("Non unicode paths operating systems cannot run BPXP")
    }
}
