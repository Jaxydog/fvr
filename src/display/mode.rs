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

//! Implements Unix file mode displays.

use std::io::{Result, StdoutLock};
use std::os::unix::fs::MetadataExt;

use super::{Show, ShowData};
use crate::arguments::model::Arguments;
use crate::{optionally_vector, optionally_vector_color};

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
    #[must_use]
    pub const fn test<const BITS: u32>(mode: u32) -> bool {
        (mode & self::MASK) == BITS
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
    #[must_use]
    pub const fn test<const MASK: u32, const BITS: u32>(mode: u32) -> bool {
        (mode & MASK) & BITS != 0
    }
}

/// Renders a file entry's Unix mode.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    /// Whether to use extended permissions.
    extended: bool,
}

impl Mode {
    /// Creates a new [`Mode`].
    pub const fn new(extended: bool) -> Self {
        Self { extended }
    }

    /// Returns the file type character.
    const fn file_type(mode: u32) -> char {
        match mode & self::file_type::MASK {
            self::file_type::SOCKET => 's',
            self::file_type::SYMBOLIC_LINK => 'l',
            self::file_type::FILE => '-',
            self::file_type::BLOCK_DEVICE => 'b',
            self::file_type::DIRECTORY => 'd',
            self::file_type::CHARACTER_DEVICE => 'c',
            self::file_type::FIFO_PIPE => 'p',
            _ => '?',
        }
    }

    /// Returns the permissions characters.
    const fn file_permissions(mode: u32) -> [[char; 3]; 4] {
        use self::permissions::{
            EXECUTE, MASK_EXTRA, MASK_GROUP, MASK_OTHER, MASK_OWNER, READ, SETGID, SETUID, STICKY, WRITE, test,
        };

        [
            [
                if test::<MASK_EXTRA, SETUID>(mode) { 'u' } else { '-' },
                if test::<MASK_EXTRA, SETGID>(mode) { 'g' } else { '-' },
                if test::<MASK_EXTRA, STICKY>(mode) { 's' } else { '-' },
            ],
            [
                if test::<MASK_OWNER, READ>(mode) { 'r' } else { '-' },
                if test::<MASK_OWNER, WRITE>(mode) { 'w' } else { '-' },
                if test::<MASK_OWNER, EXECUTE>(mode) { 'x' } else { '-' },
            ],
            [
                if test::<MASK_GROUP, READ>(mode) { 'r' } else { '-' },
                if test::<MASK_GROUP, WRITE>(mode) { 'w' } else { '-' },
                if test::<MASK_GROUP, EXECUTE>(mode) { 'x' } else { '-' },
            ],
            [
                if test::<MASK_OTHER, READ>(mode) { 'r' } else { '-' },
                if test::<MASK_OTHER, WRITE>(mode) { 'w' } else { '-' },
                if test::<MASK_OTHER, EXECUTE>(mode) { 'x' } else { '-' },
            ],
        ]
    }
}

impl Show for Mode {
    fn show_plain(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let mode = entry.data.map_or(0, MetadataExt::mode);
        let permissions = Self::file_permissions(mode).map(|s| s.map(|c| c as u8));
        let permissions = if self.extended { permissions.as_flattened() } else { permissions[1 ..].as_flattened() };

        optionally_vector!(f, [&[b'[', Self::file_type(mode) as u8], permissions, b"]"])
    }

    fn show_color(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let mode = entry.data.map_or(0, MetadataExt::mode);
        let file_type = Self::file_type(mode);
        let permissions = Self::file_permissions(mode).map(|s| s.map(|c| c as u8));
        let permissions = if self.extended { permissions.as_flattened() } else { permissions[1 ..].as_flattened() };

        optionally_vector_color!(f, BrightBlack, [b"["])?;

        match file_type {
            's' => optionally_vector_color!(f, BrightGreen, [&[file_type as u8]]),
            'l' => optionally_vector_color!(f, BrightCyan, [&[file_type as u8]]),
            '-' => optionally_vector_color!(f, BrightWhite, [&[file_type as u8]]),
            'b' => optionally_vector_color!(f, BrightRed, [&[file_type as u8]]),
            'd' => optionally_vector_color!(f, BrightBlue, [&[file_type as u8]]),
            'c' => optionally_vector_color!(f, BrightMagenta, [&[file_type as u8]]),
            'p' => optionally_vector_color!(f, BrightYellow, [&[file_type as u8]]),
            '?' => optionally_vector_color!(f, BrightBlack, [&[file_type as u8]]),
            _ => unreachable!(),
        }?;

        for permission in permissions {
            match permission {
                b'r' => optionally_vector_color!(f, BrightYellow, [&[*permission]]),
                b'w' => optionally_vector_color!(f, BrightRed, [&[*permission]]),
                b'x' => optionally_vector_color!(f, BrightGreen, [&[*permission]]),
                b'-' => optionally_vector_color!(f, BrightBlack, [&[*permission]]),
                b'u' => optionally_vector_color!(f, BrightBlue, [&[*permission]]),
                b'g' => optionally_vector_color!(f, BrightCyan, [&[*permission]]),
                b's' => optionally_vector_color!(f, BrightMagenta, [&[*permission]]),
                _ => unreachable!(),
            }?;
        }

        optionally_vector_color!(f, BrightBlack, [b"]"])
    }
}
