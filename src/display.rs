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

//! Provides custom display implementations for various types of file entry data.

use std::io::StdoutLock;

use crate::arguments::model::{Arguments, ColorChoice};

pub mod mode;
pub mod name;

/// A value that should be rendered into the terminal.
pub trait Rendered {
    /// Outputs this value into the given output stream.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be output.
    fn show(&self, arguments: &Arguments, f: &mut StdoutLock) -> std::io::Result<()> {
        match arguments.color {
            ColorChoice::Auto => {
                if supports_color::on_cached(supports_color::Stream::Stdout).is_some_and(|v| v.has_basic) {
                    self.show_color(arguments, f)
                } else {
                    self.show_plain(arguments, f)
                }
            }
            ColorChoice::Always => self.show_color(arguments, f),
            ColorChoice::Never => self.show_plain(arguments, f),
        }
    }

    /// Outputs this value into the given output stream with color.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be output.
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock) -> std::io::Result<()>;

    /// Outputs this value into the given output stream without color.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be output.
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock) -> std::io::Result<()>;
}

/// Outputs the given list of slices using [`write_all_vectored`][0] if possible.
///
/// # Examples
///
/// ```
/// optionally_vector!(&mut std::io::stdout().lock(), [
///     b"a slice of bytes,",
///     b"followed by another slice of bytes",
/// ])
/// .expect("writing failed");
/// ```
///
/// [0]: std::io::Write::write_all_vectored
#[macro_export]
macro_rules! optionally_vector {
    ($f:ident, [$($slice:expr),* $(,)?]) => {
        if <_ as ::std::io::Write>::is_write_vectored($f) {
            <_ as ::std::io::Write>::write_all_vectored($f, &mut [$(
                ::std::io::IoSlice::new($slice)
            ),*])
        } else {
            $(<_ as ::std::io::Write>::write_all($f, $slice)?;)*

            ::std::io::Result::Ok(())
        }
    };
}
