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

//! A set of helpers to manipulate BPX string sections.

mod error;

use std::{
    fs::DirEntry,
    io::{Read, Seek, SeekFrom},
    path::Path,
    string::String,
};

use elsa::FrozenMap;
pub use error::{Error, PathError};

use crate::core::{AutoSectionData, Container, Handle, SectionData};
use crate::core::header::SECTION_TYPE_STRING;
use crate::core::options::{Checksum, CompressionMethod, SectionOptions};

/// Helper class to manage a BPX string section.
///
/// # Examples
///
/// ```
/// use bpx::core::Container;
/// use bpx::core::header::{SectionHeader, Struct};
/// use bpx::strings::StringSection;
/// use bpx::util::new_byte_buf;
///
/// let mut file = Container::create(new_byte_buf(0));
/// let section = file.sections_mut().create(SectionHeader::new());
/// let mut strings = StringSection::new(section);
/// let offset = strings.put(&mut file, "Test").unwrap();
/// let str = strings.get(&mut file, offset).unwrap();
/// assert_eq!(str, "Test");
/// ```
pub struct StringSection {
    section: Handle,
    cache: FrozenMap<u32, String>,
}

impl StringSection {
    /// Creates a new string section in the given BPX container.
    ///
    /// # Arguments
    ///
    /// * `container`: the BPX container.
    ///
    /// returns: StringSection
    pub fn create<T>(container: &mut Container<T>) -> Self {
        let handle = container.sections_mut().create(
            SectionOptions::default()
                .checksum(Checksum::Weak)
                .compression(CompressionMethod::Zlib)
                .ty(SECTION_TYPE_STRING),
        );
        Self::new(handle)
    }

    /// Create a new string section from a handle.
    ///
    /// # Arguments
    ///
    /// * `hdl`: handle to the string section.
    ///
    /// returns: StringSection
    pub fn new(section: Handle) -> StringSection {
        StringSection {
            section,
            cache: FrozenMap::new(),
        }
    }

    /// Reads a string from the section.
    ///
    /// # Arguments
    ///
    /// * `container`: the BPX container.
    /// * `address`: the offset to the start of the string.
    ///
    /// returns: Result<&str, Error>
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the string could not be read or the
    /// section is corrupted/truncated.
    pub fn get<T>(&self, container: &Container<T>, address: u32) -> Result<&str, Error> {
        if self.cache.get(&address).is_none() {
            let mut section = container.sections().open(self.section)?;
            let s = low_level_read_string(address, &mut section)?;
            self.cache.insert(address, s);
        }
        Ok(unsafe { self.cache.get(&address).unwrap_unchecked() })
    }

    /// Writes a new string into the section.
    ///
    /// # Arguments
    ///
    /// * `container`: the BPX container.
    /// * `s`: the string to write.
    ///
    /// # Errors
    ///
    /// Returns an [Error] if the string could not be written.
    pub fn put<T>(&self, container: &Container<T>, s: &str) -> Result<u32, Error> {
        let mut section = container.sections().open(self.section)?;
        let address = low_level_write_string(s, &mut *section)?;
        self.cache.insert(address, String::from(s));
        Ok(address)
    }

    /// Returns the section handle.
    pub fn handle(&self) -> Handle {
        self.section
    }
}

/// Ensures string section is loaded. This is used to enable lazy loading on BPX types.
///
/// # Arguments
///
/// * `container`: the container which owns the string section.
/// * `strings`: a reference to the string section.
///
/// returns: Result<(), Error>
///
/// # Errors
///
/// Returns an [Error](crate::core::error::Error) if the section couldn't be loaded.
pub fn load_string_section<T: Read + Seek>(
    container: &Container<T>,
    strings: &StringSection,
) -> Result<(), crate::core::error::Error> {
    container.sections().load(strings.handle())?;
    Ok(())
}

fn low_level_read_string(ptr: u32, string_section: &mut AutoSectionData) -> Result<String, Error> {
    let mut curs: Vec<u8> = Vec::new();
    let mut chr: [u8; 1] = [0; 1]; //read char by char with a buffer

    string_section.seek(SeekFrom::Start(ptr as u64))?;
    // Read is enough as Sections are guaranteed to fill the buffer as much as possible
    if string_section.read(&mut chr)? != 1 {
        return Err(Error::Eos);
    }
    while chr[0] != 0x0 {
        curs.push(chr[0]);
        if string_section.read(&mut chr)? != 1 {
            return Err(Error::Eos);
        }
    }
    match String::from_utf8(curs) {
        Err(_) => Err(Error::Utf8),
        Ok(v) => Ok(v),
    }
}

fn low_level_write_string(
    s: &str,
    string_section: &mut dyn SectionData,
) -> Result<u32, std::io::Error> {
    let ptr = string_section.size() as u32;
    string_section.seek(SeekFrom::End(0))?;
    string_section.write_all(s.as_bytes())?;
    string_section.write_all(&[0x0])?;
    Ok(ptr)
}

/// Returns the file name as a UTF-8 string from a rust Path.
///
/// # Arguments
///
/// * `path`: the rust [Path].
///
/// returns: Result<String, Error>
///
/// # Errors
///
/// Returns Err if the path does not have a file name.
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
pub fn get_name_from_path(path: &Path) -> Result<&str, PathError> {
    match path.file_name() {
        Some(v) => match v.to_str() {
            Some(v) => Ok(v),
            None => Err(PathError::Utf8),
        },
        None => Err(PathError::Directory),
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
pub fn get_name_from_dir_entry(entry: &DirEntry) -> Result<String, PathError> {
    match entry.file_name().to_str() {
        Some(v) => Ok(v.into()),
        None => Err(PathError::Utf8),
    }
}
