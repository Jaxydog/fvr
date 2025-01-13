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

use std::io::{Result, Write};
use std::rc::Rc;

use crate::arguments::model::ColorChoice;
use crate::files::Entry;

pub mod mode;
pub mod name;
pub mod size;
pub mod time;
pub mod tree;
pub mod user;

/// A section of data that should be displayed to the terminal.
#[must_use = "section implementations do nothing unless you call `write`"]
pub trait Section {
    /// Writes this section into the given writer.
    ///
    /// # Errors
    ///
    /// This function will return an error if the section fails to write for any reason.
    fn write_plain<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()>;

    /// Writes this section into the given writer using color.
    ///
    /// # Errors
    ///
    /// This function will return an error if the section fails to write for any reason.
    fn write_color<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()>;

    /// Writes this section into the given writer, determining whether to use color based on the given [`ColorChoice`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the section fails to write for any reason.
    fn write<W: Write>(&self, color: ColorChoice, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        use supports_color::{Stream, on_cached};

        if color.is_always() || (color.is_auto() && { on_cached(Stream::Stdout).is_some_and(|v| v.has_basic) }) {
            self.write_color(f, parents, entry)
        } else {
            self.write_plain(f, parents, entry)
        }
    }
}

/// Returns a slice of bytes that correspond to the given color when output.
///
/// # Examples
///
/// ```
/// color_bytes!(BrightRed);
/// ```
#[macro_export]
macro_rules! color_bytes {
    ($color:ident) => {
        <::owo_colors::colors::$color as ::owo_colors::Color>::ANSI_FG.as_bytes()
    };
}

/// Writes a series of bytes into the given buffer, using vectored writing if possible.
///
/// # Examples
///
/// ```
/// writev!(f, [b"some bytes", b"and more bytes"])?;
/// writev!(f, [b"and even more bytes"] in BrightRed)
/// ```
#[macro_export]
macro_rules! writev {
    ($f:ident, [$slice:expr]) => {
        <_ as ::std::io::Write>::write_all($f, $slice)
    };
    ($f:ident, [$($slice:expr),* $(,)?]) => {
        if <_ as ::std::io::Write>::is_write_vectored($f) {
            <_ as ::std::io::Write>::write_all_vectored($f, &mut [$(::std::io::IoSlice::new($slice)),*])
        } else {
            $(<_ as ::std::io::Write>::write_all($f, $slice)?;)*

            ::std::io::Result::Ok(())
        }
    };
    ($f:ident, [$($slice:expr),* $(,)?] in $color:ident) => {
        $crate::writev!($f, [$crate::color_bytes!($color), $($slice,)* $crate::color_bytes!(Default)])
    };
}
