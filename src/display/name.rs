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

use std::io::{Result, StdoutLock, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use owo_colors::OwoColorize;

use super::Rendered;
use super::mode::permissions::EXECUTE;
use crate::arguments::model::Arguments;
use crate::optionally_vector;

/// Renders a file entry's Unix mode.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Name<'e> {
    /// The inner path name.
    path: &'e Path,
    /// Whether to resolve symbolic links.
    resolve_symlinks: bool,
    /// Whether to trim file paths.
    trim_paths: bool,
}

impl<'e> Name<'e> {
    /// Creates a new [`Name`].
    pub const fn new(path: &'e Path, resolve_symlinks: bool) -> Self {
        Self { path, resolve_symlinks, trim_paths: true }
    }
}

impl Rendered for Name<'_> {
    #[expect(clippy::only_used_in_recursion, reason = "cannot change signature as this is a trait impl")]
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock) -> Result<()> {
        let is_hidden = self.path.file_name().is_some_and(|v| v.to_string_lossy().starts_with('.'));
        let path = if self.trim_paths {
            self.path.file_name().or_else(|| self.path.parent().map(Path::as_os_str)).unwrap_or_default()
        } else {
            self.path.as_os_str()
        };

        if self.path.is_dir() {
            if is_hidden {
                write!(f, "{}", format_args!("{}/", path.to_string_lossy()).blue())?;
            } else {
                write!(f, "{}", format_args!("{}/", path.to_string_lossy()).bright_blue())?;
            }
        } else if self.path.is_file() && self.path.symlink_metadata().is_ok_and(|m| m.mode() & EXECUTE != 0) {
            if is_hidden {
                write!(f, "{}", format_args!("{}*", path.to_string_lossy()).green())?;
            } else {
                write!(f, "{}", format_args!("{}*", path.to_string_lossy()).bright_green())?;
            }
        } else if is_hidden {
            write!(f, "{}", path.to_string_lossy().bright_black())?;
        } else {
            f.write_all(path.as_encoded_bytes())?;
        };

        if self.resolve_symlinks && self.path.is_symlink() {
            let resolved = std::fs::read_link(self.path)?;

            if resolved.try_exists()? {
                write!(f, " {} ", "->".bright_cyan())?;
            } else {
                write!(f, " {} ", "~>".bright_red())?;
            };

            let relative = crate::files::relativize(self.path, &resolved).unwrap_or(resolved);

            Name { path: &relative, resolve_symlinks: false, trim_paths: false }.show_color(arguments, f)?;
        }

        Ok(())
    }

    #[expect(clippy::only_used_in_recursion, reason = "cannot change signature as this is a trait impl")]
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock) -> Result<()> {
        let path = if self.trim_paths {
            self.path.file_name().or_else(|| self.path.parent().map(Path::as_os_str)).unwrap_or_default()
        } else {
            self.path.as_os_str()
        };

        let additional: Option<&[u8]> = if self.path.is_dir() {
            Some(b"/")
        } else if self.path.is_file() && self.path.symlink_metadata().is_ok_and(|m| m.mode() & EXECUTE != 0) {
            Some(b"*")
        } else {
            None
        };
        let additional = additional.unwrap_or(&[]);

        if self.resolve_symlinks && self.path.is_symlink() {
            let resolved = std::fs::read_link(self.path)?;
            let arrow = if resolved.try_exists()? { b" -> " } else { b" ~> " };
            let relative = crate::files::relativize(self.path, &resolved).unwrap_or(resolved);

            optionally_vector!(f, [path.as_encoded_bytes(), additional, arrow])?;

            Name { path: &relative, resolve_symlinks: false, trim_paths: false }.show_plain(arguments, f)
        } else {
            optionally_vector!(f, [path.as_encoded_bytes(), additional])
        }
    }
}
