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

use std::fs::Metadata;
use std::io::{Result, StdoutLock, Write};
use std::path::Path;

use crate::arguments::model::{Arguments, ColorChoice};

pub mod mode;
pub mod name;
pub mod size;
pub mod time;
pub mod user;

/// The data provided to a [`Show`] call.
#[derive(Clone, Copy, Debug)]
pub struct ShowData<'p> {
    /// The entry path.
    pub path: &'p Path,
    /// The entry metadata, if available.
    pub data: Option<&'p Metadata>,
    /// The current entry's index.
    pub index: usize,
    /// The total number of entries.
    pub count: usize,
    /// The current depth, if this is used in a recursive call.
    pub depth: Option<usize>,
}

/// A value that should be shown to the terminal.
pub trait Show {
    /// Outputs this value into the given output stream.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be output.
    fn show(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        match arguments.color {
            ColorChoice::Auto => {
                if supports_color::on_cached(supports_color::Stream::Stdout).is_some_and(|v| v.has_basic) {
                    self.show_color(arguments, f, entry)
                } else {
                    self.show_plain(arguments, f, entry)
                }
            }
            ColorChoice::Always => self.show_color(arguments, f, entry),
            ColorChoice::Never => self.show_plain(arguments, f, entry),
        }
        .and_then(|()| f.write_all(b" "))
    }

    /// Outputs this value into the given output stream with no color.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be output.
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()>;

    /// Outputs this value into the given output stream with color.
    ///
    /// # Errors
    ///
    /// This function will return an error if the data could not be output.
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()>;
}

impl Show for [&dyn Show] {
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        self.iter().try_for_each(|show| show.show_plain(arguments, f, entry))
    }

    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        self.iter().try_for_each(|show| show.show_color(arguments, f, entry))
    }
}

impl<T: Show> Show for Option<T> {
    #[inline]
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        self.as_ref().map_or(Ok(()), |v| v.show_plain(arguments, f, entry))
    }

    #[inline]
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        self.as_ref().map_or(Ok(()), |v| v.show_color(arguments, f, entry))
    }
}

impl<T: Show> Show for [T] {
    #[inline]
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        self.iter().try_for_each(|show| show.show_plain(arguments, f, entry))
    }

    #[inline]
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        self.iter().try_for_each(|show| show.show_color(arguments, f, entry))
    }
}

impl<T: Show> Show for &[T] {
    #[inline]
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        <[T] as Show>::show_plain(self, arguments, f, entry)
    }

    #[inline]
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        <[T] as Show>::show_color(self, arguments, f, entry)
    }
}

impl<T: Show> Show for &mut [T] {
    #[inline]
    fn show_plain(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        <[T] as Show>::show_plain(self, arguments, f, entry)
    }

    #[inline]
    fn show_color(&self, arguments: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        <[T] as Show>::show_color(self, arguments, f, entry)
    }
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

/// Outputs the given list of slices using [`write_all_vectored`][0] if possible, while also applying a color style.
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
macro_rules! optionally_vector_color {
    ($f:ident, $color:ident, [$($slice:expr),* $(,)?]) => {
        $crate::optionally_vector!($f, [
            <::owo_colors::colors::$color as ::owo_colors::Color>::ANSI_FG.as_bytes(),
            $($slice,)*
            <::owo_colors::colors::Default as ::owo_colors::Color>::ANSI_FG.as_bytes(),
        ])
    };
}
