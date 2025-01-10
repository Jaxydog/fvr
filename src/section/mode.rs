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

//! Implements a section that displays an entry's file type and permissions.

use std::io::{Result, Write};
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;

use super::Section;
use crate::files::Entry;
use crate::writev;

/// Defines constants related to file entry types.
pub mod file_type {
    /// A bit mask that isolates an entry's type from its mode integer.
    pub const MASK: u32 = 0o170_000;

    /// The bit pattern for a socket.
    pub const SOCKET: u32 = 0o140_000;
    /// The bit pattern for a symbolic link.
    pub const SYMBOLIC_LINK: u32 = 0o120_000;
    /// The bit pattern for a file.
    pub const FILE: u32 = 0o100_000;
    /// The bit pattern for a block device.
    pub const BLOCK_DEVICE: u32 = 0o060_000;
    /// The bit pattern for a directory.
    pub const DIRECTORY: u32 = 0o040_000;
    /// The bit pattern for a character device.
    pub const CHARACTER_DEVICE: u32 = 0o020_000;
    /// The bit pattern for a pipe.
    pub const FIFO_PIPE: u32 = 0o010_000;

    /// Returns `true` if the given mode's file type is set to `BITS`.
    #[inline]
    #[must_use]
    pub const fn test<const BITS: u32>(mode: u32) -> bool {
        (mode & self::MASK) == BITS
    }

    /// Returns the `set` value if the file type matches, otherwise returns `unset`.
    #[inline]
    pub const fn test_map<'t, T: ?Sized, const BITS: u32>(mode: u32, set: &'t T, unset: &'t T) -> &'t T {
        if self::test::<BITS>(mode) { set } else { unset }
    }
}

/// Defines constants related to file entry permissions.
pub mod permissions {
    /// A bit mask that isolates an entry's permissions from its mode integer.
    pub const MASK: u32 = 0o7_777;
    /// A bit mask that isolates an entry's extra permissions from its mode integer.
    pub const MASK_EXTRA: u32 = 0o7_000;
    /// A bit mask that isolates an entry's owner permissions from its mode integer.
    pub const MASK_OWNER: u32 = 0o0_700;
    /// A bit mask that isolates an entry's group permissions from its mode integer.
    pub const MASK_GROUP: u32 = 0o0_070;
    /// A bit mask that isolates an entry's other permissions from its mode integer.
    pub const MASK_OTHER: u32 = 0o0_007;

    /// The bit pattern for read permissions.
    pub const READ: u32 = 0o0_444;
    /// The bit pattern for write permissions.
    pub const WRITE: u32 = 0o0_222;
    /// The bit pattern for execute permissions.
    pub const EXECUTE: u32 = 0o0_111;
    /// The bit pattern for set user ID permissions.
    pub const SETUID: u32 = 0o4_000;
    /// The bit pattern for set group ID permissions.
    pub const SETGID: u32 = 0o2_000;
    /// The bit pattern for "sticky" permissions.
    pub const STICKY: u32 = 0o1_000;

    /// Returns `true` if the given mode's permissions contain `BITS`.
    #[inline]
    #[must_use]
    pub const fn test<const MASK: u32, const BITS: u32>(mode: u32) -> bool {
        (mode & MASK) & BITS != 0
    }

    /// Returns the `set` value if the permission is set, otherwise returns `unset`.
    #[inline]
    pub const fn test_map<'t, T: ?Sized, const MASK: u32, const BITS: u32>(
        mode: u32,
        set: &'t T,
        unset: &'t T,
    ) -> &'t T {
        if self::test::<MASK, BITS>(mode) { set } else { unset }
    }
}

/// A [`Section`] that writes an entry's file type and permissions.
#[derive(Clone, Copy, Debug)]
pub struct ModeSection {
    /// Whether to use an extended permission format.
    pub extended: bool,
}

impl ModeSection {
    /// The byte used to represent an empty permission.
    pub const PERM_EMPTY: u8 = b'-';
    /// The byte used to represent an execute permission.
    pub const PERM_EXECUTE: u8 = b'x';
    /// The byte used to represent a read permission.
    pub const PERM_READ: u8 = b'r';
    /// The byte used to represent a `setgid` permission.
    pub const PERM_SETGID: u8 = b'g';
    /// The byte used to represent a `setuid` permission.
    pub const PERM_SETUID: u8 = b'u';
    /// The byte used to represent a read permission.
    pub const PERM_STICKY: u8 = b's';
    /// The byte used to represent a write permission.
    pub const PERM_WRITE: u8 = b'w';
    /// The byte used to represent a block device.
    pub const TYPE_BLOCK_DEVICE: u8 = b'b';
    /// The byte used to represent a character device.
    pub const TYPE_CHARACTER_DEVICE: u8 = b'c';
    /// The byte used to represent a directory.
    pub const TYPE_DIRECTORY: u8 = b'd';
    /// The byte used to represent a pipe.
    pub const TYPE_FIFO_PIPE: u8 = b'p';
    /// The byte used to represent a file.
    pub const TYPE_FILE: u8 = b'-';
    /// The byte used to represent a socket.
    pub const TYPE_SOCKET: u8 = b's';
    /// The byte used to represent a symbolic link.
    pub const TYPE_SYMBOLIC_LINK: u8 = b'l';
    /// The byte used to represent an unknown file type.
    pub const TYPE_UNKNOWN: u8 = b'?';

    /// Returns a series of bytes that represent the file type for the given mode.
    #[must_use]
    pub const fn get_type(mode: u32) -> u8 {
        use self::file_type::{
            BLOCK_DEVICE, CHARACTER_DEVICE, DIRECTORY, FIFO_PIPE, FILE, MASK, SOCKET, SYMBOLIC_LINK,
        };

        match mode & MASK {
            FILE => Self::TYPE_FILE,
            DIRECTORY => Self::TYPE_DIRECTORY,
            SYMBOLIC_LINK => Self::TYPE_SYMBOLIC_LINK,
            FIFO_PIPE => Self::TYPE_FIFO_PIPE,
            SOCKET => Self::TYPE_SOCKET,
            BLOCK_DEVICE => Self::TYPE_BLOCK_DEVICE,
            CHARACTER_DEVICE => Self::TYPE_CHARACTER_DEVICE,
            _ => Self::TYPE_UNKNOWN,
        }
    }

    /// Returns a series of bytes that represent the permissions for the given mode.
    #[must_use]
    pub const fn get_permissions(mode: u32) -> [u8; 12] {
        use self::permissions::{
            EXECUTE, MASK_EXTRA, MASK_GROUP, MASK_OTHER, MASK_OWNER, READ, SETGID, SETUID, STICKY, WRITE, test_map,
        };

        [
            *test_map::<_, MASK_EXTRA, SETUID>(mode, &Self::PERM_SETUID, &Self::PERM_EMPTY),
            *test_map::<_, MASK_EXTRA, SETGID>(mode, &Self::PERM_SETGID, &Self::PERM_EMPTY),
            *test_map::<_, MASK_EXTRA, STICKY>(mode, &Self::PERM_SETUID, &Self::PERM_EMPTY),
            *test_map::<_, MASK_OWNER, READ>(mode, &Self::PERM_READ, &Self::PERM_EMPTY),
            *test_map::<_, MASK_OWNER, WRITE>(mode, &Self::PERM_WRITE, &Self::PERM_EMPTY),
            *test_map::<_, MASK_OWNER, EXECUTE>(mode, &Self::PERM_EXECUTE, &Self::PERM_EMPTY),
            *test_map::<_, MASK_GROUP, READ>(mode, &Self::PERM_READ, &Self::PERM_EMPTY),
            *test_map::<_, MASK_GROUP, WRITE>(mode, &Self::PERM_WRITE, &Self::PERM_EMPTY),
            *test_map::<_, MASK_GROUP, EXECUTE>(mode, &Self::PERM_EXECUTE, &Self::PERM_EMPTY),
            *test_map::<_, MASK_OTHER, READ>(mode, &Self::PERM_READ, &Self::PERM_EMPTY),
            *test_map::<_, MASK_OTHER, WRITE>(mode, &Self::PERM_WRITE, &Self::PERM_EMPTY),
            *test_map::<_, MASK_OTHER, EXECUTE>(mode, &Self::PERM_EXECUTE, &Self::PERM_EMPTY),
        ]
    }
}

impl Section for ModeSection {
    fn write_plain<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let mode = entry.data.map(MetadataExt::mode).unwrap_or_default();
        let permissions = Self::get_permissions(mode);

        writev!(f, [&[b'[', Self::get_type(mode)], if self.extended { &permissions } else { &permissions[3 ..] }, b"]"])
    }

    fn write_color<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        writev!(f, [b"["] in BrightBlack)?;

        let mode = entry.data.map(MetadataExt::mode).unwrap_or_default();

        match Self::get_type(mode) {
            v @ Self::TYPE_FILE => writev!(f, [&[v]] in BrightWhite)?,
            v @ Self::TYPE_DIRECTORY => writev!(f, [&[v]] in BrightBlue)?,
            v @ Self::TYPE_SYMBOLIC_LINK => writev!(f, [&[v]] in BrightCyan)?,
            v @ Self::TYPE_FIFO_PIPE => writev!(f, [&[v]] in BrightYellow)?,
            v @ Self::TYPE_SOCKET => writev!(f, [&[v]] in BrightGreen)?,
            v @ Self::TYPE_BLOCK_DEVICE => writev!(f, [&[v]] in BrightRed)?,
            v @ Self::TYPE_CHARACTER_DEVICE => writev!(f, [&[v]] in BrightMagenta)?,
            v @ Self::TYPE_UNKNOWN => writev!(f, [&[v]] in BrightBlack)?,
            _ => unreachable!(),
        }

        let permissions = Self::get_permissions(mode);

        for permission in if self.extended { &permissions } else { &permissions[3 ..] } {
            match *permission {
                v @ Self::PERM_EMPTY => writev!(f, [&[v]] in BrightBlack)?,
                v @ Self::PERM_READ => writev!(f, [&[v]] in BrightYellow)?,
                v @ Self::PERM_WRITE => writev!(f, [&[v]] in BrightRed)?,
                v @ Self::PERM_EXECUTE => writev!(f, [&[v]] in BrightGreen)?,
                v @ Self::PERM_SETGID => writev!(f, [&[v]] in BrightCyan)?,
                v @ Self::PERM_SETUID => writev!(f, [&[v]] in BrightBlue)?,
                v @ Self::PERM_STICKY => writev!(f, [&[v]] in BrightMagenta)?,
                _ => unreachable!(),
            }
        }

        writev!(f, [b"]"] in BrightBlack)
    }
}
