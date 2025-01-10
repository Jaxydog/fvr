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

//! Implements sections related to entry names.

use std::io::{Result, Write};
use std::rc::Rc;

use super::Section;
use crate::files::Entry;
use crate::writev;

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
}

impl Section for NameSection {
    fn write_plain<W: Write>(&self, f: &mut W, parents: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let name = if self.trim_paths { entry.path.file_name() } else { None }.unwrap_or(entry.path.as_os_str());

        if entry.is_dir() {
            writev!(f, [name.as_encoded_bytes(), Self::DIR_SUFFIX])?;
        } else if entry.is_file() && entry.is_executable() {
            writev!(f, [name.as_encoded_bytes(), Self::EXE_SUFFIX])?;
        } else {
            writev!(f, [name.as_encoded_bytes()])?;
        };

        if self.resolve_symlinks && entry.is_symlink() { SymlinkSection.write_plain(f, parents, entry) } else { Ok(()) }
    }

    fn write_color<W: Write>(&self, f: &mut W, parents: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let name = (if self.trim_paths { entry.path.file_name() } else { None }).unwrap_or(entry.path.as_os_str());
        let name = name.as_encoded_bytes();

        if entry.is_symlink() {
            if entry.is_hidden() { writev!(f, [name] in Cyan) } else { writev!(f, [name] in BrightCyan) }?;

            if self.resolve_symlinks { SymlinkSection.write_color(f, parents, entry) } else { Ok(()) }
        } else if entry.is_dir() {
            if entry.is_hidden() { writev!(f, [name] in Blue) } else { writev!(f, [name] in BrightBlue) }?;

            writev!(f, [Self::DIR_SUFFIX] in White)
        } else if entry.is_executable() {
            if entry.is_hidden() { writev!(f, [name] in Green) } else { writev!(f, [name] in BrightGreen) }?;

            writev!(f, [Self::EXE_SUFFIX] in White)
        } else {
            // We purposefully do not color the name for non-hidden files since uncolored text is brighter than white
            // for some terminal themes, and leaving it as such makes it easier to differentiate.
            if entry.is_hidden() { writev!(f, [name] in BrightBlack) } else { writev!(f, [name]) }
        }
    }
}

/// A [`Section`] that writes an entry's resolved symbolic link.
#[derive(Clone, Copy, Debug)]
pub struct SymlinkSection;

impl SymlinkSection {
    /// The arrow used when a symbolic link is broken.
    pub const BROKEN_ARROW: &[u8] = b"~>";
    /// The arrow used when a symbolic link is valid.
    pub const LINKED_ARROW: &[u8] = b"->";
}

impl Section for SymlinkSection {
    fn write_plain<W: Write>(&self, f: &mut W, parents: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let resolved = std::fs::read_link(entry.path)?;

        if resolved.try_exists()? {
            writev!(f, [b" ", Self::LINKED_ARROW, b" "])?;
        } else {
            writev!(f, [b" ", Self::BROKEN_ARROW, b" "])?;
        }

        let data = std::fs::symlink_metadata(&resolved).ok();
        let path = crate::files::relativize(entry.path, &resolved).unwrap_or(resolved);
        let entry = Entry::root(path.as_ref(), data.as_ref());

        NameSection { trim_paths: false, resolve_symlinks: false }.write_plain(f, parents, &Rc::new(entry))
    }

    fn write_color<W: Write>(&self, f: &mut W, parents: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let resolved = std::fs::read_link(entry.path)?;

        if resolved.try_exists()? {
            writev!(f, [b" ", Self::LINKED_ARROW, b" "] in BrightBlack)?;
        } else {
            writev!(f, [b" ", Self::BROKEN_ARROW, b" "] in BrightRed)?;
        }

        let data = std::fs::symlink_metadata(&resolved).ok();
        let path = crate::files::relativize(entry.path, &resolved).unwrap_or(resolved);
        let entry = Entry::root(path.as_ref(), data.as_ref());

        NameSection { trim_paths: false, resolve_symlinks: false }.write_color(f, parents, &Rc::new(entry))
    }
}
