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

//! Implements a section that displays an entry's filetype and permissions.

use std::io::{Result, StdoutLock};
use std::path::Path;

use recomposition::filter::Filter;

use super::Section;
use crate::files::{Entry, EntryMetadata, Filetype, Permissions};
use crate::{color_bytes, writev};

/// A [`Section`] that writes an entry's filetype and permissions.
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
    /// The byte used to represent an unknown filetype.
    pub const TYPE_UNKNOWN: u8 = b'?';

    /// Creates a new [`ModeSection`].
    #[inline]
    #[must_use]
    pub const fn new(extended: bool) -> Self {
        Self { extended }
    }

    /// Returns the ASCII flag character for the filetype.
    const fn get_filetype_flag(filetype: Filetype) -> u8 {
        use crate::files::{
            FILETYPE_BLOCK_DEVICE, FILETYPE_CHARACTER_DEVICE, FILETYPE_DIRECTORY, FILETYPE_FIFO_PIPE, FILETYPE_FILE,
            FILETYPE_SOCKET, FILETYPE_SYMBOLIC_LINK,
        };

        match filetype.get() {
            FILETYPE_FILE => Self::TYPE_FILE,
            FILETYPE_DIRECTORY => Self::TYPE_DIRECTORY,
            FILETYPE_SYMBOLIC_LINK => Self::TYPE_SYMBOLIC_LINK,
            FILETYPE_FIFO_PIPE => Self::TYPE_FIFO_PIPE,
            FILETYPE_SOCKET => Self::TYPE_SOCKET,
            FILETYPE_BLOCK_DEVICE => Self::TYPE_BLOCK_DEVICE,
            FILETYPE_CHARACTER_DEVICE => Self::TYPE_CHARACTER_DEVICE,
            _ => Self::TYPE_UNKNOWN,
        }
    }

    /// Returns the ASCII flag characters for each permission.
    const fn get_permission_flags(permissions: Permissions) -> [u8; 12] {
        use crate::files::{
            PERMISSION_EXECUTE, PERMISSION_READ, PERMISSION_SET_GID, PERMISSION_SET_UID, PERMISSION_STICKY,
            PERMISSION_WRITE,
        };

        [
            if permissions.has_extra(PERMISSION_SET_UID) { Self::PERM_SETUID } else { Self::PERM_EMPTY },
            if permissions.has_extra(PERMISSION_SET_GID) { Self::PERM_SETGID } else { Self::PERM_EMPTY },
            if permissions.has_extra(PERMISSION_STICKY) { Self::PERM_STICKY } else { Self::PERM_EMPTY },
            if permissions.has_owner(PERMISSION_READ) { Self::PERM_READ } else { Self::PERM_EMPTY },
            if permissions.has_owner(PERMISSION_WRITE) { Self::PERM_WRITE } else { Self::PERM_EMPTY },
            if permissions.has_owner(PERMISSION_EXECUTE) { Self::PERM_EXECUTE } else { Self::PERM_EMPTY },
            if permissions.has_group(PERMISSION_READ) { Self::PERM_READ } else { Self::PERM_EMPTY },
            if permissions.has_group(PERMISSION_WRITE) { Self::PERM_WRITE } else { Self::PERM_EMPTY },
            if permissions.has_group(PERMISSION_EXECUTE) { Self::PERM_EXECUTE } else { Self::PERM_EMPTY },
            if permissions.has_other(PERMISSION_READ) { Self::PERM_READ } else { Self::PERM_EMPTY },
            if permissions.has_other(PERMISSION_WRITE) { Self::PERM_WRITE } else { Self::PERM_EMPTY },
            if permissions.has_other(PERMISSION_EXECUTE) { Self::PERM_EXECUTE } else { Self::PERM_EMPTY },
        ]
    }
}

impl Section for ModeSection {
    fn write_plain<F>(&self, f: &mut StdoutLock<'_>, _: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let permissions = entry.data.as_ref().map_or_default(EntryMetadata::permissions);
        let permissions = Self::get_permission_flags(permissions);
        let permissions = if self.extended { &permissions } else { &permissions[3 ..] };

        let filetype = entry.data.as_ref().map_or_default(EntryMetadata::filetype);
        let filetype = Self::get_filetype_flag(filetype);

        writev!(f, [&[b'[', filetype], permissions, b"]"])
    }

    fn write_color<F>(&self, f: &mut StdoutLock<'_>, _: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        writev!(f, [b"["] in White)?;

        let filetype = entry.data.as_ref().map_or_default(EntryMetadata::filetype);

        match Self::get_filetype_flag(filetype) {
            v @ Self::TYPE_DIRECTORY => writev!(f, [&[v]] in BrightBlue)?,
            v @ Self::TYPE_SYMBOLIC_LINK => writev!(f, [&[v]] in BrightCyan)?,
            v @ Self::TYPE_FIFO_PIPE => writev!(f, [&[v]] in BrightYellow)?,
            v @ Self::TYPE_SOCKET => writev!(f, [&[v]] in BrightGreen)?,
            v @ Self::TYPE_BLOCK_DEVICE => writev!(f, [&[v]] in BrightRed)?,
            v @ Self::TYPE_CHARACTER_DEVICE => writev!(f, [&[v]] in BrightMagenta)?,
            v @ (Self::TYPE_FILE | Self::TYPE_UNKNOWN) => writev!(f, [&[v]] in BrightBlack)?,
            _ => unreachable!(),
        }

        let permissions = entry.data.as_ref().map_or_default(EntryMetadata::permissions);
        let permissions = Self::get_permission_flags(permissions);
        let permissions = if self.extended { &permissions } else { &permissions[3 ..] };

        let mut buffer = Vec::<u8>::with_capacity(permissions.len() * 6);

        for permission in permissions {
            buffer.extend_from_slice(match *permission {
                Self::PERM_EMPTY => color_bytes!(BrightBlack),
                Self::PERM_READ => color_bytes!(BrightYellow),
                Self::PERM_WRITE => color_bytes!(BrightRed),
                Self::PERM_EXECUTE => color_bytes!(BrightGreen),
                Self::PERM_SETGID => color_bytes!(BrightCyan),
                Self::PERM_SETUID => color_bytes!(BrightBlue),
                Self::PERM_STICKY => color_bytes!(BrightMagenta),
                _ => unreachable!(),
            });

            buffer.push(*permission);
        }

        writev!(f, [&buffer])?;
        writev!(f, [b"]"] in White)
    }
}
