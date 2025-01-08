// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 Jaxydog
//
// This file is part of fvr.
//
// fvr is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// fvr is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with fvr. If not,
// see <https://www.gnu.org/licenses/>.

//! Implements entry file name displays.

use std::fs::Metadata;
use std::io::{Result, StdoutLock};

use super::{Show, ShowData};
use crate::arguments::model::Arguments;
use crate::files::{is_executable, is_hidden};
use crate::{optionally_vector, optionally_vector_color};

/// Renders a file entry's Unix mode.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct Name {
    /// Whether to resolve symbolic links.
    resolve_symlinks: bool,
    /// Whether to trim file paths.
    trim_paths: bool,
}

impl Name {
    /// Creates a new [`Name`].
    pub const fn new(resolve_symlinks: bool, trim_paths: bool) -> Self {
        Self { resolve_symlinks, trim_paths }
    }
}

#[expect(clippy::only_used_in_recursion, reason = "we still need to call the function")]
impl Show for Name {
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let file_name = if self.trim_paths {
            entry.path.file_name().unwrap_or(entry.path.as_os_str())
        } else {
            entry.path.as_os_str()
        };

        let executable = entry.data.is_some_and(is_executable);
        let additional = if entry.data.map_or_else(|| entry.path.is_dir(), Metadata::is_dir) {
            Some::<&[u8]>(b"/")
        } else if entry.data.map_or_else(|| entry.path.is_file(), Metadata::is_file) && executable {
            Some::<&[u8]>(b"*")
        } else {
            None
        }
        .unwrap_or(&[]);

        if self.resolve_symlinks && entry.data.map_or_else(|| entry.path.is_symlink(), Metadata::is_symlink) {
            let resolved = std::fs::read_link(entry.path)?;
            let metadata = std::fs::symlink_metadata(&resolved).ok();

            let arrow = if resolved.try_exists()? { b" -> " } else { b" ~> " };
            let relative = crate::files::relativize(entry.path, &resolved).unwrap_or(resolved);

            optionally_vector!(f, [file_name.as_encoded_bytes(), additional, arrow])?;

            let entry = ShowData { path: &relative, data: metadata.as_ref(), ..entry };

            Self { resolve_symlinks: false, trim_paths: false }.show_plain(arguments, f, entry)
        } else {
            optionally_vector!(f, [file_name.as_encoded_bytes(), additional])
        }
    }

    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let file_name = if self.trim_paths {
            entry.path.file_name().unwrap_or(entry.path.as_os_str())
        } else {
            entry.path.as_os_str()
        };

        let executable = entry.data.is_some_and(is_executable);
        let hidden = is_hidden(entry.path);

        if entry.data.map_or_else(|| entry.path.is_symlink(), Metadata::is_symlink) {
            if hidden {
                optionally_vector_color!(f, Cyan, [file_name.as_encoded_bytes()])?;
            } else {
                optionally_vector_color!(f, BrightCyan, [file_name.as_encoded_bytes()])?;
            }

            if self.resolve_symlinks {
                let resolved = std::fs::read_link(entry.path)?;
                let metadata = std::fs::symlink_metadata(&resolved).ok();

                if resolved.try_exists()? {
                    optionally_vector_color!(f, BrightRed, [b" -> "])?;
                } else {
                    optionally_vector_color!(f, BrightRed, [b" ~> "])?;
                };

                let relative = crate::files::relativize(entry.path, &resolved).unwrap_or(resolved);
                let entry = ShowData { path: &relative, data: metadata.as_ref(), ..entry };

                Self { resolve_symlinks: false, trim_paths: false }.show_color(arguments, f, entry)?;
            }

            Ok(())
        } else if entry.data.map_or_else(|| entry.path.is_dir(), Metadata::is_dir) {
            if hidden {
                optionally_vector_color!(f, Blue, [file_name.as_encoded_bytes(), b"/"])
            } else {
                optionally_vector_color!(f, BrightBlue, [file_name.as_encoded_bytes(), b"/"])
            }
        } else if entry.data.map_or_else(|| entry.path.is_file(), Metadata::is_file) && executable {
            if hidden {
                optionally_vector_color!(f, Green, [file_name.as_encoded_bytes(), b"*"])
            } else {
                optionally_vector_color!(f, BrightGreen, [file_name.as_encoded_bytes(), b"*"])
            }
        } else if hidden {
            optionally_vector_color!(f, BrightBlack, [file_name.as_encoded_bytes()])
        } else {
            optionally_vector_color!(f, White, [file_name.as_encoded_bytes()])
        }
    }
}
