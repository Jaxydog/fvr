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

//! Implements entry file size displays.

use std::io::{Result, StdoutLock};
use std::os::unix::fs::MetadataExt;

use super::{Show, ShowData};
use crate::arguments::model::{Arguments, SizeVisibility};
use crate::{optionally_vector, optionally_vector_color};

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

    /// Returns the given size converted to a human-readable unit.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "the size will never be big enough to lose meaningful precision")]
    pub const fn get_base_2(size: u64) -> (f64, Unit<3>) {
        match size {
            v if v < KIBIBYTES.divisor => (size as f64, BYTES_2),
            v if v < MEBIBYTES.divisor => (size as f64 / KIBIBYTES.divisor as f64, KIBIBYTES),
            v if v < GIBIBYTES.divisor => (size as f64 / MEBIBYTES.divisor as f64, MEBIBYTES),
            v if v < TEBIBYTES.divisor => (size as f64 / GIBIBYTES.divisor as f64, GIBIBYTES),
            v if v < PEBIBYTES.divisor => (size as f64 / TEBIBYTES.divisor as f64, TEBIBYTES),
            v if v < EXBIBYTES.divisor => (size as f64 / PEBIBYTES.divisor as f64, PEBIBYTES),
            _ => (size as f64 / EXBIBYTES.divisor as f64, EXBIBYTES),
        }
    }

    /// Returns the given size converted to a human-readable unit.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "the size will never be big enough to lose meaningful precision")]
    pub const fn get_base_10(size: u64) -> (f64, Unit<2>) {
        match size {
            v if v < KILOBYTES.divisor => (size as f64, BYTES_10),
            v if v < MEGABYTES.divisor => (size as f64 / KILOBYTES.divisor as f64, KILOBYTES),
            v if v < GIGABYTES.divisor => (size as f64 / MEGABYTES.divisor as f64, MEGABYTES),
            v if v < TERABYTES.divisor => (size as f64 / GIGABYTES.divisor as f64, GIGABYTES),
            v if v < PETABYTES.divisor => (size as f64 / TERABYTES.divisor as f64, TERABYTES),
            v if v < EXABYTES.divisor => (size as f64 / PETABYTES.divisor as f64, PETABYTES),
            _ => (size as f64 / EXABYTES.divisor as f64, EXABYTES),
        }
    }

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
        #[must_use]
        pub const fn new(suffix: &'static [u8; N], divisor: u64) -> Self {
            Self { suffix, divisor }
        }
    }
}

/// Renders a file entry's size.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    /// Determines whether to display file sizes.
    visibility: SizeVisibility,
}

impl Size {
    /// Files below this are considered 'medium'.
    pub const MEDIUM_THRESHOLD: u64 = 50 * (self::units::MEBIBYTES.divisor);
    /// Files below this are considered 'small'.
    pub const SMALL_THRESHOLD: u64 = 50 * (self::units::KIBIBYTES.divisor);

    /// Creates a new [`Size`].
    pub const fn new(visibility: SizeVisibility) -> Self {
        Self { visibility }
    }
}

impl Show for Size {
    fn show_plain(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        if entry.is_dir() {
            return match self.visibility {
                SizeVisibility::Hide => unreachable!(),
                SizeVisibility::Simple => {
                    optionally_vector!(f, [b"-", &[b' '; 19]])
                }
                SizeVisibility::Base2 => {
                    optionally_vector!(f, [&[b' '; 5], b"- -", &[b' '; 2]])
                }
                SizeVisibility::Base10 => {
                    optionally_vector!(f, [&[b' '; 5], b"- -", &[b' '; 1]])
                }
            };
        }

        match self.visibility {
            SizeVisibility::Hide => unreachable!(),
            SizeVisibility::Simple => {
                let mut buffer = itoa::Buffer::new();
                let bytes = buffer.format(entry.data.map_or(0, MetadataExt::size)).as_bytes();
                let padding = &[b' '; 20][.. 20 - bytes.len()];

                optionally_vector!(f, [bytes, padding])
            }
            SizeVisibility::Base2 => {
                let (size, unit) = self::units::get_base_2(entry.data.map_or(0, MetadataExt::size));
                let mut buffer = ryu::Buffer::new();
                let Some((start, end)) = buffer.format(size).as_bytes().split_once(|b| *b == b'.') else {
                    unreachable!();
                };

                let padding = &[b' '; 6][.. 6 - (start.len() + 2)];

                optionally_vector!(f, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
            }
            SizeVisibility::Base10 => {
                let (size, unit) = self::units::get_base_10(entry.data.map_or(0, MetadataExt::size));
                let mut buffer = ryu::Buffer::new();
                let Some((start, end)) = buffer.format(size).as_bytes().split_once(|b| *b == b'.') else {
                    unreachable!();
                };

                let padding = &[b' '; 6][.. 6 - (start.len() + 2)];

                optionally_vector!(f, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
            }
        }
    }

    fn show_color(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        if entry.is_dir() {
            return match self.visibility {
                SizeVisibility::Hide => unreachable!(),
                SizeVisibility::Simple => {
                    optionally_vector_color!(f, BrightBlack, [b"-", &[b' '; 19]])
                }
                SizeVisibility::Base2 => {
                    optionally_vector_color!(f, BrightBlack, [&[b' '; 3], b"-.- -", &[b' '; 2]])
                }
                SizeVisibility::Base10 => {
                    optionally_vector_color!(f, BrightBlack, [&[b' '; 3], b"-.- -", &[b' '; 1]])
                }
            };
        }

        let size = entry.data.map_or(0, MetadataExt::size);

        match self.visibility {
            SizeVisibility::Hide => unreachable!(),
            SizeVisibility::Simple => {
                let mut buffer = itoa::Buffer::new();
                let bytes = buffer.format(size).as_bytes();
                let padding = &[b' '; 20][.. 20 - bytes.len()];

                match size {
                    _ if size <= Self::SMALL_THRESHOLD => optionally_vector_color!(f, BrightGreen, [bytes, padding]),
                    _ if size <= Self::MEDIUM_THRESHOLD => optionally_vector_color!(f, BrightYellow, [bytes, padding]),
                    _ => optionally_vector_color!(f, BrightRed, [bytes, padding]),
                }
            }
            SizeVisibility::Base2 => {
                let (scaled_size, unit) = self::units::get_base_2(size);
                let mut buffer = ryu::Buffer::new();
                let Some((start, end)) = buffer.format(scaled_size).as_bytes().split_once(|b| *b == b'.') else {
                    unreachable!();
                };

                let padding = &[b' '; 6][.. 6 - (start.len() + 2)];

                match scaled_size {
                    _ if size <= Self::SMALL_THRESHOLD => {
                        optionally_vector_color!(f, BrightGreen, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
                    }
                    _ if size <= Self::MEDIUM_THRESHOLD => {
                        optionally_vector_color!(f, BrightYellow, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
                    }
                    _ => {
                        optionally_vector_color!(f, BrightRed, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
                    }
                }
            }
            SizeVisibility::Base10 => {
                let (scaled_size, unit) = self::units::get_base_10(size);
                let mut buffer = ryu::Buffer::new();
                let Some((start, end)) = buffer.format(scaled_size).as_bytes().split_once(|b| *b == b'.') else {
                    unreachable!();
                };

                let padding = &[b' '; 6][.. 6 - (start.len() + 2)];

                match scaled_size {
                    _ if size <= Self::SMALL_THRESHOLD => {
                        optionally_vector_color!(f, BrightGreen, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
                    }
                    _ if size <= Self::MEDIUM_THRESHOLD => {
                        optionally_vector_color!(f, BrightYellow, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
                    }
                    _ => {
                        optionally_vector_color!(f, BrightRed, [padding, start, b".", &end[.. 1], b" ", unit.suffix])
                    }
                }
            }
        }
    }
}
