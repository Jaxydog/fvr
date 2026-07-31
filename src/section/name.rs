// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025–2026 Jaxydog
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

//! Implements sections related to entry names.

use std::borrow::Cow;
use std::io::{ErrorKind, Result, StdoutLock};
use std::path::Path;

use recomposition::filter::Filter;

use super::Section;
use crate::files::{Entry, EntryMetadata};
use crate::{color_bytes, writev};

/// A [`Section`] that writes an entry's name.
#[derive(Clone, Copy, Debug)]
pub struct NameSection {
    /// Whether to trim the entry to just its name or to render the full path.
    pub trim_paths: bool,
    /// Whether to resolve the actual path of symbolic links.
    pub resolve_symlinks: bool,
}

impl NameSection {
    /// The suffix used for directories.
    pub const DIR_SUFFIX: &[u8] = b"/";
    /// The suffix used for executable files.
    pub const EXE_SUFFIX: &[u8] = b"*";
    /// The suffix used for symbolic links.
    pub const SYMLINK_SUFFIX: &[u8] = b"@";

    /// Creates a new [`NameSection`].
    #[inline]
    #[must_use]
    pub const fn new(trim_paths: bool, resolve_symlinks: bool) -> Self {
        Self { trim_paths, resolve_symlinks }
    }
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
            Self::SYMLINK_SUFFIX
        } else if entry.is_dir() && !name.eq_ignore_ascii_case(b"/") {
            Self::DIR_SUFFIX
        } else if entry.is_file() && entry.is_executable() {
            Self::EXE_SUFFIX
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
            Self::SYMLINK_SUFFIX => {
                if entry.is_hidden() {
                    color_bytes!(Cyan)
                } else {
                    color_bytes!(BrightCyan)
                }
            }
            Self::DIR_SUFFIX => {
                if entry.is_hidden() {
                    color_bytes!(Blue)
                } else {
                    color_bytes!(BrightBlue)
                }
            }
            Self::EXE_SUFFIX => {
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

impl SymlinkSection {
    /// The arrow used when a symbolic link is broken.
    pub const BROKEN_ARROW: &[u8] = b"-/>";
    /// The arrow used when a symbolic link is valid.
    pub const LINKED_ARROW: &[u8] = b"-->";
    /// The arrow used when a symbolic link is recursive.
    pub const RECURSIVE_ARROW: &[u8] = b"<->";
}

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
            Ok(data) => (Some(EntryMetadata::new(&data)), Self::LINKED_ARROW, color_bytes!(White)),
            Err(error) if error.kind() == ErrorKind::NotFound => (None, Self::BROKEN_ARROW, color_bytes!(BrightRed)),

            Err(error) if error.kind() != ErrorKind::FilesystemLoop => return Err(error),
            Err(_) => {
                if color {
                    writev!(f, [b" ", Self::RECURSIVE_ARROW, b" "] in Cyan)?;
                } else {
                    writev!(f, [b" ", Self::RECURSIVE_ARROW, b" "])?;
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
