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

use super::DEFAULT_MAX_DEPTH;
use crate::util::macros::{create_options, open_options};
use crate::{
    package::{Architecture, Platform},
    sd::Value,
};

/// The required settings to create a new BPXP.
///
/// *This is intended to be generated with help of [CreateOptions](CreateOptions).*
#[derive(Clone)]
pub struct Settings {
    /// The package target architecture.
    pub architecture: Architecture,

    /// The package target platform/OS.
    pub platform: Platform,

    /// The package metadata (stored as a BPXSD [Object](crate::sd::Object)).
    pub metadata: Option<Value>,

    /// The package type code.
    pub type_code: [u8; 2],
}

/// Specific cofniguration options for BPXP.
pub struct Options {
    /// Defines the maximum depth of the metadata BPXSD object.
    pub max_depth: usize,
}

create_options! {
    /// Utility to simplify generation of [Settings](Settings) required when creating a new BPXP.
    CreateOptions {
        settings: Settings = Settings {
            architecture: Architecture::Any,
            platform: Platform::Any,
            metadata: None,
            type_code: [0x50, 0x48],
        },
        max_depth: usize = DEFAULT_MAX_DEPTH
    }
}

impl<T> CreateOptions<T> {
    /// Defines the CPU architecture that the package is targeting.
    ///
    /// *By default, no CPU architecture is targeted.*
    ///
    /// # Arguments
    ///
    /// * `arch`: the CPU architecture this package is designed to work on.
    pub fn architecture(mut self, arch: Architecture) -> Self {
        self.settings.architecture = arch;
        self
    }

    /// Defines the platform that the package is targeting.
    ///
    /// *By default, no platform is targeted.*
    ///
    /// # Arguments
    ///
    /// * `platform`: the platform this package is designed to work on.
    pub fn platform(mut self, platform: Platform) -> Self {
        self.settings.platform = platform;
        self
    }

    /// Defines the metadata for the package.
    ///
    /// *By default, no metadata object is set.*
    ///
    /// # Arguments
    ///
    /// * `val`: the BPXSD metadata value.
    pub fn metadata(mut self, val: Value) -> Self {
        self.settings.metadata = Some(val);
        self
    }

    /// Defines the type of the package.
    ///
    /// *By default, the package variant is 'PK' to identify
    /// a package designed for FPKG.*
    ///
    /// # Arguments
    ///
    /// * `type_code`: the type code of this package.
    pub fn type_code(mut self, type_code: [u8; 2]) -> Self {
        self.settings.type_code = type_code;
        self
    }

    /// Defines the maximum depth of the metadata BPXSD object.
    ///
    /// *By default, the maximum depth is set to [DEFAULT_MAX_DEPTH](DEFAULT_MAX_DEPTH).*
    ///
    /// # Arguments
    ///
    /// * `max_depth`: the maximum depth of the metadata BPXSD object.
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }
}

impl<T: std::io::Seek> From<(T, Settings)> for CreateOptions<T> {
    fn from((backend, settings): (T, Settings)) -> Self {
        Self {
            options: crate::core::options::CreateOptions::new(backend),
            settings,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

open_options! {
    /// Utility to allow configuring the underlying BPX container when opening a BPXP.
    OpenOptions {
        max_depth: usize = DEFAULT_MAX_DEPTH
    }
}

impl<T> OpenOptions<T> {
    /// Defines the maximum depth of the metadata BPXSD object.
    ///
    /// *By default, the maximum depth is set to [DEFAULT_MAX_DEPTH](DEFAULT_MAX_DEPTH).*
    ///
    /// # Arguments
    ///
    /// * `max_depth`: the maximum depth of the metadata BPXSD object.
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }
}
