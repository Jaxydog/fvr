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

use std::io::{StdoutLock, Write};

use owo_colors::OwoColorize;

use super::Rendered;
use crate::arguments::model::Arguments;
use crate::optionally_vector;

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
    /// The inner mode integer.
    mode: u32,
    /// Whether to use extended permissions.
    extended: bool,
}

impl Mode {
    /// Creates a new [`Mode`].
    pub const fn new(mode: u32, extended: bool) -> Self {
        Self { mode, extended }
    }

    /// Returns the file type character.
    const fn file_type(self) -> char {
        match self.mode & self::file_type::MASK {
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
    const fn file_permissions(self) -> [[char; 3]; 4] {
        use self::permissions::{
            EXECUTE, MASK_EXTRA, MASK_GROUP, MASK_OTHER, MASK_OWNER, READ, SETGID, SETUID, STICKY, WRITE, test,
        };

        [
            [
                if test::<MASK_EXTRA, SETUID>(self.mode) { 'u' } else { '-' },
                if test::<MASK_EXTRA, SETGID>(self.mode) { 'g' } else { '-' },
                if test::<MASK_EXTRA, STICKY>(self.mode) { 's' } else { '-' },
            ],
            [
                if test::<MASK_OWNER, READ>(self.mode) { 'r' } else { '-' },
                if test::<MASK_OWNER, WRITE>(self.mode) { 'w' } else { '-' },
                if test::<MASK_OWNER, EXECUTE>(self.mode) { 'x' } else { '-' },
            ],
            [
                if test::<MASK_GROUP, READ>(self.mode) { 'r' } else { '-' },
                if test::<MASK_GROUP, WRITE>(self.mode) { 'w' } else { '-' },
                if test::<MASK_GROUP, EXECUTE>(self.mode) { 'x' } else { '-' },
            ],
            [
                if test::<MASK_OTHER, READ>(self.mode) { 'r' } else { '-' },
                if test::<MASK_OTHER, WRITE>(self.mode) { 'w' } else { '-' },
                if test::<MASK_OTHER, EXECUTE>(self.mode) { 'x' } else { '-' },
            ],
        ]
    }
}

impl Rendered for Mode {
    fn show_color(&self, _: &Arguments, f: &mut StdoutLock) -> std::io::Result<()> {
        write!(f, "{}", '['.bright_black())?;

        match self.file_type() {
            c @ 's' => write!(f, "{}", c.bright_green()),
            c @ 'l' => write!(f, "{}", c.bright_cyan()),
            c @ '-' => write!(f, "{}", c.bright_white()),
            c @ 'b' => write!(f, "{}", c.bright_red()),
            c @ 'd' => write!(f, "{}", c.bright_blue()),
            c @ 'c' => write!(f, "{}", c.bright_purple()),
            c @ 'p' => write!(f, "{}", c.bright_yellow()),
            c @ '?' => write!(f, "{}", c.bright_black()),
            _ => unreachable!(),
        }?;

        let permissions = self.file_permissions();
        let permissions = if self.extended { permissions.as_flattened() } else { permissions[1 ..].as_flattened() };

        for permission in permissions {
            match permission {
                c @ 'r' => write!(f, "{}", c.bright_yellow()),
                c @ 'w' => write!(f, "{}", c.bright_red()),
                c @ 'x' => write!(f, "{}", c.bright_green()),
                c @ '-' => write!(f, "{}", c.bright_black()),
                c @ 'u' => write!(f, "{}", c.bright_blue()),
                c @ 'g' => write!(f, "{}", c.bright_cyan()),
                c @ 's' => write!(f, "{}", c.bright_purple()),
                _ => unreachable!(),
            }?;
        }

        write!(f, "{}", ']'.bright_black())
    }

    fn show_plain(&self, _: &Arguments, f: &mut StdoutLock) -> std::io::Result<()> {
        let permissions = self.file_permissions().map(|s| s.map(|c| c as u8));
        let permissions = if self.extended { permissions.as_flattened() } else { permissions[1 ..].as_flattened() };

        optionally_vector!(f, [&[b'[', self.file_type() as u8], permissions, b"]"])
    }
}
