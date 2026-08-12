// SPDX-License-Identifier: GPL-3.0-or-later
//
// Copyright © 2025–2026 Jaxydog
//
// This file is part of fvr.
//
// fvr is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
//
// fvr is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with fvr. If not,
// see <https://www.gnu.org/licenses/>.

//! Implements sections related to entry names.

use std::borrow::Cow;
use std::io::{ErrorKind, Result, StdoutLock};
use std::path::Path;

use recomposition::filter::Filter;

use super::Section;
use crate::files::{Entry, EntryMetadata};
use crate::{color_bytes, writev};

/// The suffix used for directories.
const DIRECTORY_SUFFIX: &[u8] = b"/";
/// The suffix used for executable files.
const EXECUTABLE_SUFFIX: &[u8] = b"*";
/// The suffix used for symbolic links.
const SYMBOLIC_LINK_SUFFIX: &[u8] = b"@";

/// The arrow used when a symbolic link is broken.
const BROKEN_ARROW: &[u8] = b"-/>";
/// The arrow used when a symbolic link is valid.
const LINKED_ARROW: &[u8] = b"-->";
/// The arrow used when a symbolic link is recursive.
const RECURSIVE_ARROW: &[u8] = b"<->";

/// A [`Section`] that writes an entry's name.
#[derive(Clone, Copy, Debug)]
pub struct NameSection {
    /// Whether to trim the entry to just its name or to render the full path.
    pub trim_paths: bool,
    /// Whether to resolve the actual path of symbolic links.
    pub resolve_symlinks: bool,
}

impl Section for NameSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, parents: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let name = entry.path.file_name().filter(|_| self.trim_paths).unwrap_or_else(|| {
            // This is so that the directory suffix is only ever written once.
            entry.path.trim_trailing_sep().as_os_str()
        });
        let name = name.as_encoded_bytes();

        let suffix = if entry.is_symlink() {
            SYMBOLIC_LINK_SUFFIX
        } else if entry.is_dir() && name != b"/" {
            DIRECTORY_SUFFIX
        } else if entry.is_file() && entry.is_executable() {
            EXECUTABLE_SUFFIX
        } else {
            &[]
        };

        if !color {
            writev!(f, [name, suffix])?;

            if self.resolve_symlinks && entry.is_symlink() {
                SymlinkSection.write(color, f, parents, entry)?;
            }

            return Ok(());
        }

        let entry_color = match suffix {
            SYMBOLIC_LINK_SUFFIX => {
                if entry.is_hidden() {
                    color_bytes!(Cyan)
                } else {
                    color_bytes!(BrightCyan)
                }
            }
            DIRECTORY_SUFFIX => {
                if entry.is_hidden() {
                    color_bytes!(Blue)
                } else {
                    color_bytes!(BrightBlue)
                }
            }
            EXECUTABLE_SUFFIX => {
                if entry.is_hidden() {
                    color_bytes!(Green)
                } else {
                    color_bytes!(BrightGreen)
                }
            }
            _ => {
                if entry.is_hidden() {
                    color_bytes!(White)
                } else {
                    // We purposefully do not color the name for non-hidden files since uncolored text is brighter than
                    // white for some terminal themes, and leaving it as such makes it easier to differentiate.
                    color_bytes!(Default)
                }
            }
        };

        writev!(f, [entry_color, name, color_bytes!(White), suffix, color_bytes!(Default)])?;

        if self.resolve_symlinks && entry.is_symlink() {
            SymlinkSection.write(color, f, parents, entry)?;
        }

        Ok(())
    }
}

/// A [`Section`] that writes an entry's resolved symbolic link.
#[derive(Clone, Copy, Debug)]
pub struct SymlinkSection;

impl Section for SymlinkSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, parents: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        const NAME_SECTION: NameSection = NameSection { trim_paths: false, resolve_symlinks: false };

        let link_path = std::fs::read_link(&entry.path)?;
        let real_path = if link_path.is_relative()
            && let Some(parent) = parents.last().map(|entry| &entry.path)
        {
            Cow::Owned(parent.join(&link_path))
        } else {
            Cow::Borrowed(&link_path)
        };

        let (data, arrow, arrow_color) = match std::fs::metadata(real_path.as_ref()) {
            Ok(data) => (Some(EntryMetadata::new(&data)), LINKED_ARROW, color_bytes!(White)),
            Err(error) if error.kind() == ErrorKind::NotFound => (None, BROKEN_ARROW, color_bytes!(BrightRed)),

            Err(error) if error.kind() != ErrorKind::FilesystemLoop => return Err(error),
            Err(_) => {
                if color {
                    writev!(f, [b" ", RECURSIVE_ARROW, b" "] in Cyan)?;
                } else {
                    writev!(f, [b" ", RECURSIVE_ARROW, b" "])?;
                }

                let path = crate::files::relativize(&entry.path, &link_path).unwrap_or_else(|| link_path.clone());
                let data = std::fs::symlink_metadata(real_path.as_ref()).ok().map(|data| EntryMetadata::new(&data));
                let entry = Entry::root(path.into_boxed_path(), data, entry.filter);

                return NAME_SECTION.write(color, f, parents, &entry);
            }
        };

        if color {
            writev!(f, [b" ", arrow_color, arrow, color_bytes!(Default), b" "])?;
        } else {
            writev!(f, [b" ", arrow, b" "])?;
        }

        let path = crate::files::relativize(&entry.path, &link_path).unwrap_or(link_path);
        let entry = Entry::root(path.into_boxed_path(), data, entry.filter);

        NAME_SECTION.write(color, f, parents, &entry)
    }
}
