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

//! Utility generation macros mainly designed for BPX and IO related error types.

#[macro_export]
/// Automatically implements conversion from the given set of error types.
macro_rules! impl_err_conversion {
    ($self: ident { $($foreign: ty => $variant: ident),* }) => {
        $(
            impl From<$foreign> for $self
            {
                fn from(e: $foreign) -> Self
                {
                    return $self::$variant(e);
                }
            }
        )*
    };
}

#[macro_export]
/// Generates an enum where each variant is assigned a &'static str name.
macro_rules! named_enum {
    (
        $(#[$enum_outer:meta])* $name: ident { $($(#[$outer:meta])* $variant: ident : $namestr: expr),* }
    ) => {
        $(#[$enum_outer])*
        #[derive(Debug)]
        pub enum $name
        {
            $(
                $(#[$outer])*
                $variant
            ),*
        }

        impl $name
        {
            /// Returns the string corresponding to the current enum variant.
            pub fn name(&self) -> &'static str
            {
                return match self {
                    $(Self::$variant => $namestr),*
                };
            }
        }
    };
}

macro_rules! create_options {
    ($(#[$options_outer:meta])* CreateOptions {
        $($field_name: ident : $field_type: ty = $field_default: expr),*
    }) => {
        $(#[$options_outer])*
        pub struct CreateOptions<T> {
            pub(crate) options: crate::core::options::CreateOptions<T>,
            $(
                pub(crate) $field_name: $field_type
            ),*
        }

        impl<T> CreateOptions<T> {
            /// Creates a new set of options for a BPX container.
            ///
            /// # Arguments
            ///
            /// * `backend`: the IO backend to be associated with the container.
            pub fn new(backend: T) -> CreateOptions<T> {
                CreateOptions {
                    options: crate::core::options::CreateOptions::new(backend),
                    $(
                        $field_name: $field_default
                    ),*
                }
            }

            /// Sets the maximum size of a section allowed to fit in RAM in bytes.
            ///
            /// The default is set to [DEFAULT_MEMORY_THRESHOLD](crate::core::DEFAULT_MEMORY_THRESHOLD) bytes.
            ///
            /// # Arguments
            ///
            /// * `size`: the maximum size of a section in RAM.
            pub fn memory_threshold(mut self, size: u32) -> Self {
                self.options = self.options.memory_threshold(size);
                self
            }

            /// Reverts the file when a save operation fails to keep the BPX unchanged/unmodified after
            /// a save failure.
            ///
            /// This works by saving the container to a temporary storage before overwriting the original
            /// IO backend. The default for this option is set to false.
            ///
            /// # Arguments
            ///
            /// * `flag`: true to revert the file on save failure, false otherwise.
            pub fn revert_on_save_failure(mut self, flag: bool) -> Self {
                self.options = self.options.revert_on_save_failure(flag);
                self
            }
        }

        impl<T: std::io::Seek> From<T> for CreateOptions<T> {
            fn from(value: T) -> Self {
                Self::new(value)
            }
        }
    };
}

macro_rules! open_options {
    ($(#[$options_outer:meta])* OpenOptions {
        $($field_name: ident : $field_type: ty = $field_default: expr),*
    }) => {
        $(#[$options_outer])*
        pub struct OpenOptions<T> {
            pub(crate) options: crate::core::options::OpenOptions<T>,
            $(
                pub(crate) $field_name: $field_type
            ),*
        }

        impl<T> OpenOptions<T> {
            /// Creates a new set of options for a BPX container.
            ///
            /// # Arguments
            ///
            /// * `backend`: the IO backend to be associated with the container.
            pub fn new(backend: T) -> OpenOptions<T> {
                OpenOptions {
                    options: crate::core::options::OpenOptions::new(backend),
                    $(
                        $field_name: $field_default
                    ),*
                }
            }

            /// Disable signature checks when loading the container.
            ///
            /// The default is set to false.
            ///
            /// # Arguments
            ///
            /// * `flag`: true to skip signature checks, false otherwise.
            pub fn skip_signature(mut self, flag: bool) -> Self {
                self.options = self.options.skip_signature(flag);
                self
            }

            /// Skip BPX version checks.
            ///
            /// The default is set to false.
            ///
            /// # Arguments
            ///
            /// * `flag`: true to skip version checks, false otherwise.
            pub fn skip_versions(mut self, flag: bool) -> Self {
                self.options = self.options.skip_versions(flag);
                self
            }

            /// Disable checksum checks when loading the section header/table or a section.
            ///
            /// The default is set to false.
            ///
            /// # Arguments
            ///
            /// * `flag`: true to skip checksum checks on load, false otherwise.
            pub fn skip_checksum(mut self, flag: bool) -> Self {
                self.options = self.options.skip_checksum(flag);
                self
            }

            /// Sets the maximum size of a section allowed to fit in RAM in bytes.
            ///
            /// The default is set to [DEFAULT_MEMORY_THRESHOLD](crate::core::DEFAULT_MEMORY_THRESHOLD) bytes.
            ///
            /// # Arguments
            ///
            /// * `size`: the maximum size of a section in RAM.
            pub fn memory_threshold(mut self, size: u32) -> Self {
                self.options = self.options.memory_threshold(size);
                self
            }

            /// Reverts the file when a save operation fails to keep the BPX unchanged/unmodified after
            /// a save failure.
            ///
            /// This works by saving the container to a temporary storage before overwriting the original
            /// IO backend. The default for this option is set to false.
            ///
            /// # Arguments
            ///
            /// * `flag`: true to revert the file on save failure, false otherwise.
            pub fn revert_on_save_failure(mut self, flag: bool) -> Self {
                self.options = self.options.revert_on_save_failure(flag);
                self
            }
        }

        impl<T: std::io::Seek> From<T> for OpenOptions<T> {
            fn from(value: T) -> Self {
                Self::new(value)
            }
        }
    };
}

pub(crate) use create_options;
pub(crate) use open_options;

pub use impl_err_conversion;
pub use named_enum;
