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

/// The byte used to represent a block device.
const FILETYPE_FLAG_BLOCK_DEVICE: u8 = b'b';
/// The byte used to represent a character device.
const FILETYPE_FLAG_CHARACTER_DEVICE: u8 = b'c';
/// The byte used to represent a directory.
const FILETYPE_FLAG_DIRECTORY: u8 = b'd';
/// The byte used to represent a pipe.
const FILETYPE_FLAG_FIFO_PIPE: u8 = b'p';
/// The byte used to represent a file.
const FILETYPE_FLAG_FILE: u8 = b'-';
/// The byte used to represent a socket.
const FILETYPE_FLAG_SOCKET: u8 = b's';
/// The byte used to represent a symbolic link.
const FILETYPE_FLAG_SYMBOLIC_LINK: u8 = b'l';
/// The byte used to represent an unknown filetype.
const FILETYPE_FLAG_UNKNOWN: u8 = b'?';
/// The byte used to represent an empty permission.
const PERMISSION_FLAG_EMPTY: u8 = b'-';
/// The byte used to represent an execute permission.
const PERMISSION_FLAG_EXECUTE: u8 = b'x';
/// The byte used to represent a read permission.
const PERMISSION_FLAG_READ: u8 = b'r';
/// The byte used to represent a `setgid` permission.
const PERMISSION_FLAG_SETGID: u8 = b'g';
/// The byte used to represent a `setuid` permission.
const PERMISSION_FLAG_SETUID: u8 = b'u';
/// The byte used to represent a read permission.
const PERMISSION_FLAG_STICKY: u8 = b's';
/// The byte used to represent a write permission.
const PERMISSION_FLAG_WRITE: u8 = b'w';

/// A [`Section`] that writes an entry's filetype and permissions.
#[derive(Clone, Copy, Debug)]
pub struct ModeSection {
    /// Whether to use an extended permission format.
    pub extended: bool,
}

impl ModeSection {
    /// Returns the ASCII flag character for the filetype.
    const fn get_filetype_flag(filetype: Filetype) -> u8 {
        use crate::files::{
            FILETYPE_BLOCK_DEVICE, FILETYPE_CHARACTER_DEVICE, FILETYPE_DIRECTORY, FILETYPE_FIFO_PIPE, FILETYPE_FILE,
            FILETYPE_SOCKET, FILETYPE_SYMBOLIC_LINK,
        };

        match filetype.get() {
            FILETYPE_FILE => FILETYPE_FLAG_FILE,
            FILETYPE_DIRECTORY => FILETYPE_FLAG_DIRECTORY,
            FILETYPE_SYMBOLIC_LINK => FILETYPE_FLAG_SYMBOLIC_LINK,
            FILETYPE_FIFO_PIPE => FILETYPE_FLAG_FIFO_PIPE,
            FILETYPE_SOCKET => FILETYPE_FLAG_SOCKET,
            FILETYPE_BLOCK_DEVICE => FILETYPE_FLAG_BLOCK_DEVICE,
            FILETYPE_CHARACTER_DEVICE => FILETYPE_FLAG_CHARACTER_DEVICE,
            _ => FILETYPE_FLAG_UNKNOWN,
        }
    }

    /// Returns the ASCII flag characters for each permission.
    const fn get_permission_flags(permissions: Permissions) -> [u8; 12] {
        use crate::files::{
            PERMISSION_EXECUTE, PERMISSION_READ, PERMISSION_SET_GID, PERMISSION_SET_UID, PERMISSION_STICKY,
            PERMISSION_WRITE,
        };

        [
            if permissions.has_extra(PERMISSION_SET_UID) { PERMISSION_FLAG_SETUID } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_extra(PERMISSION_SET_GID) { PERMISSION_FLAG_SETGID } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_extra(PERMISSION_STICKY) { PERMISSION_FLAG_STICKY } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_owner(PERMISSION_READ) { PERMISSION_FLAG_READ } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_owner(PERMISSION_WRITE) { PERMISSION_FLAG_WRITE } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_owner(PERMISSION_EXECUTE) { PERMISSION_FLAG_EXECUTE } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_group(PERMISSION_READ) { PERMISSION_FLAG_READ } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_group(PERMISSION_WRITE) { PERMISSION_FLAG_WRITE } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_group(PERMISSION_EXECUTE) { PERMISSION_FLAG_EXECUTE } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_other(PERMISSION_READ) { PERMISSION_FLAG_READ } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_other(PERMISSION_WRITE) { PERMISSION_FLAG_WRITE } else { PERMISSION_FLAG_EMPTY },
            if permissions.has_other(PERMISSION_EXECUTE) { PERMISSION_FLAG_EXECUTE } else { PERMISSION_FLAG_EMPTY },
        ]
    }
}

impl Section for ModeSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, _: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let filetype = Self::get_filetype_flag(entry.data.as_ref().map_or_default(EntryMetadata::filetype));
        let permissions = Self::get_permission_flags(entry.data.as_ref().map_or_default(EntryMetadata::permissions));
        let permissions = if self.extended { &permissions } else { &permissions[3 ..] };

        if !color {
            return writev!(f, [&[b'[', filetype], permissions, b"]"]);
        }

        let mut flag_buffer = Vec::with_capacity((permissions.len() + 1) * 6);

        flag_buffer.extend_from_slice(match filetype {
            FILETYPE_FLAG_DIRECTORY => color_bytes!(BrightBlue),
            FILETYPE_FLAG_SYMBOLIC_LINK => color_bytes!(BrightCyan),
            FILETYPE_FLAG_FIFO_PIPE => color_bytes!(BrightYellow),
            FILETYPE_FLAG_SOCKET => color_bytes!(BrightGreen),
            FILETYPE_FLAG_BLOCK_DEVICE => color_bytes!(BrightRed),
            FILETYPE_FLAG_CHARACTER_DEVICE => color_bytes!(BrightMagenta),
            FILETYPE_FLAG_FILE | FILETYPE_FLAG_UNKNOWN => color_bytes!(BrightBlack),
            _ => unreachable!(),
        });

        flag_buffer.push(filetype);

        for permission in permissions {
            flag_buffer.extend_from_slice(match *permission {
                PERMISSION_FLAG_EMPTY => color_bytes!(BrightBlack),
                PERMISSION_FLAG_READ => color_bytes!(BrightYellow),
                PERMISSION_FLAG_WRITE => color_bytes!(BrightRed),
                PERMISSION_FLAG_EXECUTE => color_bytes!(BrightGreen),
                PERMISSION_FLAG_SETGID => color_bytes!(BrightCyan),
                PERMISSION_FLAG_SETUID => color_bytes!(BrightBlue),
                PERMISSION_FLAG_STICKY => color_bytes!(BrightMagenta),
                _ => unreachable!(),
            });

            flag_buffer.push(*permission);
        }

        writev!(f, [color_bytes!(White), b"[", &flag_buffer, color_bytes!(White), b"]", color_bytes!(Default)])
    }
}
