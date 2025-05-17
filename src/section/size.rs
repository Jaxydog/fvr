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

//! Implements a section that displays an entry's size.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::Metadata;
use std::io::{Result, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ahash::RandomState;
use recomposition::filter::Filter;

use super::Section;
use crate::arguments::model::SizeVisibility;
use crate::files::Entry;
use crate::writev;

/// Defines human-readable units.
pub mod units {
    /// Bytes (base 2).
    pub const BYTES_2: Unit<3> = Unit::new(b"B  ", 1);
    /// Kibibytes.
    pub const KIBIBYTES: Unit<3> = Unit::new(b"KiB", BYTES_2.divisor << 10);
    /// Mebibytes.
    pub const MEBIBYTES: Unit<3> = Unit::new(b"MiB", KIBIBYTES.divisor << 10);
    /// Gibibytes.
    pub const GIBIBYTES: Unit<3> = Unit::new(b"GiB", MEBIBYTES.divisor << 10);
    /// Tebibytes.
    pub const TEBIBYTES: Unit<3> = Unit::new(b"TiB", GIBIBYTES.divisor << 10);
    /// Pebibytes.
    pub const PEBIBYTES: Unit<3> = Unit::new(b"PiB", TEBIBYTES.divisor << 10);
    /// Exbibytes.
    pub const EXBIBYTES: Unit<3> = Unit::new(b"EiB", PEBIBYTES.divisor << 10);

    /// Bytes (base 2).
    pub const BYTES_10: Unit<2> = Unit::new(b"B ", 1);
    /// Kilobytes.
    pub const KILOBYTES: Unit<2> = Unit::new(b"KB", BYTES_2.divisor * 1000);
    /// Megabytes.
    pub const MEGABYTES: Unit<2> = Unit::new(b"MB", KILOBYTES.divisor * 1000);
    /// Gigabytes.
    pub const GIGABYTES: Unit<2> = Unit::new(b"GB", MEGABYTES.divisor * 1000);
    /// Terabytes.
    pub const TERABYTES: Unit<2> = Unit::new(b"TB", GIGABYTES.divisor * 1000);
    /// Petabytes.
    pub const PETABYTES: Unit<2> = Unit::new(b"PB", TERABYTES.divisor * 1000);
    /// Exabytes.
    pub const EXABYTES: Unit<2> = Unit::new(b"EB", PETABYTES.divisor * 1000);

    /// A size unit.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Unit<const N: usize> {
        /// The unit's suffix.
        pub suffix: &'static [u8; N],
        /// The number to divide the file size by.
        pub divisor: u64,
    }

    impl<const N: usize> Unit<N> {
        /// Creates a new [`Unit`].
        #[inline]
        #[must_use]
        pub const fn new(suffix: &'static [u8; N], divisor: u64) -> Self {
            Self { suffix, divisor }
        }

        /// Converts the given size into this unit.
        #[expect(clippy::cast_precision_loss, reason = "sizes will never be big enough to lose meaningful precision")]
        #[inline]
        #[must_use]
        pub const fn convert(self, size: u64) -> f64 {
            size as f64 / self.divisor as f64
        }
    }

    /// Returns the given size converted to a human-readable unit.
    #[must_use]
    pub const fn get_base_2(size: u64) -> (f64, Unit<3>) {
        match size {
            v if v < KIBIBYTES.divisor => (BYTES_2.convert(v), BYTES_2),
            v if v < MEBIBYTES.divisor => (KIBIBYTES.convert(v), KIBIBYTES),
            v if v < GIBIBYTES.divisor => (MEBIBYTES.convert(v), MEBIBYTES),
            v if v < TEBIBYTES.divisor => (GIBIBYTES.convert(v), GIBIBYTES),
            v if v < PEBIBYTES.divisor => (TEBIBYTES.convert(v), TEBIBYTES),
            v if v < EXBIBYTES.divisor => (PEBIBYTES.convert(v), PEBIBYTES),
            v => (EXBIBYTES.convert(v), EXBIBYTES),
        }
    }

    /// Returns the given size converted to a human-readable unit.
    #[must_use]
    pub const fn get_base_10(size: u64) -> (f64, Unit<2>) {
        match size {
            v if v < KILOBYTES.divisor => (BYTES_10.convert(v), BYTES_10),
            v if v < MEGABYTES.divisor => (KILOBYTES.convert(v), KILOBYTES),
            v if v < GIGABYTES.divisor => (MEGABYTES.convert(v), MEGABYTES),
            v if v < TERABYTES.divisor => (GIGABYTES.convert(v), GIGABYTES),
            v if v < PETABYTES.divisor => (TERABYTES.convert(v), TERABYTES),
            v if v < EXABYTES.divisor => (PETABYTES.convert(v), PETABYTES),
            v => (EXABYTES.convert(v), EXABYTES),
        }
    }
}

/// A [`Section`] that writes an entry's size.
#[derive(Clone, Copy, Debug)]
pub struct SizeSection {
    /// Determines the size format to use.
    pub visibility: SizeVisibility,
}

impl SizeSection {
    /// The byte that represents a lack of size.
    pub const CHAR_BLANK: u8 = b'-';
    /// The byte that represents a decimal.
    pub const CHAR_DECIMAL: u8 = b'.';
    /// The byte used for padding.
    pub const CHAR_PADDING: u8 = b' ';
    /// Files above this are considered 'large'.
    pub const LARGE_THRESHOLD: u64 = 50 * self::units::MEBIBYTES.divisor;
    /// Files above this are considered 'medium'.
    pub const MEDIUM_THRESHOLD: u64 = 50 * self::units::KIBIBYTES.divisor;
    /// The array used to pad a base-10 string.
    pub const PAD_BASE_10: &[u8] = &[Self::CHAR_PADDING; Self::WIDTH_BASE_10];
    /// The array used to pad a base-2 string.
    pub const PAD_BASE_2: &[u8] = &[Self::CHAR_PADDING; Self::WIDTH_BASE_2];
    /// The width of a base-10 output.
    pub const WIDTH_BASE_10: usize = 8;
    /// The width of a base-2 output.
    pub const WIDTH_BASE_2: usize = 10;
    /// The width of a simple size output.
    pub const WIDTH_SIMPLE: usize = 20;

    /// Creates a new [`SizeSection`].
    #[inline]
    #[must_use]
    pub const fn new(visibility: SizeVisibility) -> Self {
        Self { visibility }
    }

    /// Returns the maximum length that all simple size sections in the given directory will take up.
    fn max_simple_len(parent: &Path) -> usize {
        thread_local! {
            static CACHE: RefCell<HashMap<Box<Path>, usize, RandomState>> = RefCell::new(HashMap::default());
        }

        CACHE.with(|cache| {
            if let Some(len) = cache.borrow().get(parent).copied() {
                return len;
            }

            let len = std::fs::read_dir(parent).ok().and_then(|v| {
                v.map_while(|v| v.and_then(|v| v.metadata()).ok())
                    .map(|v| itoa::Buffer::new().format(v.size()).len())
                    .max()
            });
            let len = len.unwrap_or(Self::WIDTH_SIMPLE);

            cache.borrow_mut().insert(Box::from(parent), len);

            len
        })
    }
}

impl Section for SizeSection {
    fn write_plain<W, F>(&self, f: &mut W, parents: &[&Rc<Entry<F>>], entry: &Rc<Entry<F>>) -> Result<()>
    where
        W: Write,
        F: Filter<(PathBuf, Metadata)>,
    {
        if entry.is_dir() {
            return match self.visibility {
                SizeVisibility::Simple => {
                    writev!(f, [&[Self::CHAR_BLANK], &vec![
                        Self::CHAR_PADDING;
                        Self::max_simple_len(parents[parents.len() - 1].path) - 1
                    ]])
                }
                SizeVisibility::Base2 => writev!(f, [
                    &[Self::CHAR_PADDING; 3],
                    &[Self::CHAR_BLANK, Self::CHAR_DECIMAL, Self::CHAR_BLANK],
                    &[Self::CHAR_PADDING, Self::CHAR_BLANK],
                    &[Self::CHAR_PADDING; 2],
                ]),
                SizeVisibility::Base10 => writev!(f, [
                    &[Self::CHAR_PADDING; 2],
                    &[Self::CHAR_BLANK, Self::CHAR_DECIMAL, Self::CHAR_BLANK],
                    &[Self::CHAR_PADDING, Self::CHAR_BLANK, Self::CHAR_PADDING],
                ]),
                SizeVisibility::Hide => unreachable!(),
            };
        }

        let size = entry.data.map_or(0, MetadataExt::size);

        if self.visibility.is_simple() {
            let mut buffer = itoa::Buffer::new();
            let bytes = buffer.format(size).as_bytes();

            let length = Self::max_simple_len(parents[parents.len() - 1].path);
            let padding = vec![Self::CHAR_PADDING; length];
            let padding = &padding[.. length - bytes.len()];

            return writev!(f, [bytes, padding]);
        }

        let (scaled_size, suffix, padding): (f64, &[u8], &[u8]) = if self.visibility.is_base2() {
            let (scaled_size, unit) = self::units::get_base_2(size);

            (scaled_size, unit.suffix, Self::PAD_BASE_2)
        } else {
            let (scaled_size, unit) = self::units::get_base_10(size);

            (scaled_size, unit.suffix, Self::PAD_BASE_10)
        };

        let mut buffer = ryu::Buffer::new();
        let bytes = buffer.format(scaled_size).as_bytes();
        let Some((whole, decimal)) = bytes.split_once(|b| *b == Self::CHAR_DECIMAL) else { unreachable!() };
        let decimal = &[Self::CHAR_DECIMAL, decimal[0], Self::CHAR_PADDING];

        let padding = &padding[.. padding.len() - (whole.len() + 3 + suffix.len())];

        writev!(f, [padding, whole, decimal, suffix])
    }

    fn write_color<W, F>(&self, f: &mut W, parents: &[&Rc<Entry<F>>], entry: &Rc<Entry<F>>) -> Result<()>
    where
        W: Write,
        F: Filter<(PathBuf, Metadata)>,
    {
        if entry.is_dir() {
            return match self.visibility {
                SizeVisibility::Simple => {
                    writev!(f, [&[Self::CHAR_BLANK], &vec![
                        Self::CHAR_PADDING;
                        Self::max_simple_len(parents[parents.len() - 1].path) - 1
                    ]] in BrightBlack)
                }
                SizeVisibility::Base2 => writev!(f, [
                    &[Self::CHAR_PADDING; 3],
                    &[Self::CHAR_BLANK, Self::CHAR_DECIMAL, Self::CHAR_BLANK],
                    &[Self::CHAR_PADDING, Self::CHAR_BLANK],
                    &[Self::CHAR_PADDING; 2],
                ] in BrightBlack),
                SizeVisibility::Base10 => writev!(f, [
                    &[Self::CHAR_PADDING; 2],
                    &[Self::CHAR_BLANK, Self::CHAR_DECIMAL, Self::CHAR_BLANK],
                    &[Self::CHAR_PADDING, Self::CHAR_BLANK, Self::CHAR_PADDING],
                ] in BrightBlack),
                SizeVisibility::Hide => unreachable!(),
            };
        }

        let size = entry.data.map_or(0, MetadataExt::size);

        if self.visibility.is_simple() {
            let mut buffer = itoa::Buffer::new();
            let bytes = buffer.format(size).as_bytes();

            let length = Self::max_simple_len(parents[parents.len() - 1].path);
            let padding = vec![Self::CHAR_PADDING; length];
            let padding = &padding[.. length - bytes.len()];

            return match size {
                v if v < Self::MEDIUM_THRESHOLD => writev!(f, [bytes, padding] in BrightGreen),
                v if v < Self::LARGE_THRESHOLD => writev!(f, [bytes, padding] in BrightYellow),
                _ => writev!(f, [bytes, padding] in BrightRed),
            };
        }

        let (scaled_size, suffix, padding): (f64, &[u8], &[u8]) = if self.visibility.is_base2() {
            let (scaled_size, unit) = self::units::get_base_2(size);

            (scaled_size, unit.suffix, Self::PAD_BASE_2)
        } else {
            let (scaled_size, unit) = self::units::get_base_10(size);

            (scaled_size, unit.suffix, Self::PAD_BASE_10)
        };

        let mut buffer = ryu::Buffer::new();
        let bytes = buffer.format(scaled_size).as_bytes();
        let Some((whole, decimal)) = bytes.split_once(|b| *b == Self::CHAR_DECIMAL) else { unreachable!() };
        let decimal = &[Self::CHAR_DECIMAL, decimal[0], Self::CHAR_PADDING];

        let padding = &padding[.. padding.len() - (whole.len() + 3 + suffix.len())];

        match size {
            v if v < Self::MEDIUM_THRESHOLD => writev!(f, [padding, whole, decimal, suffix] in BrightGreen),
            v if v < Self::LARGE_THRESHOLD => writev!(f, [padding, whole, decimal, suffix] in BrightYellow),
            _ => writev!(f, [padding, whole, decimal, suffix] in BrightRed),
        }
    }
}
