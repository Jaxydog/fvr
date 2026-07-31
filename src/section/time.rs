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

//! Implements sections related to entry timestamps.

use std::io::{Result, StdoutLock};
use std::path::Path;

use recomposition::filter::Filter;
use time::format_description::BorrowedFormatItem;
use time::format_description::well_known::Iso8601;
use time::{OffsetDateTime, SignedDuration, UtcOffset};

use super::Section;
use crate::files::{Entry, EntryMetadata};
use crate::writev;

/// The byte used when the creation date cannot be determined.
const MISSING_CHARACTER: u8 = b'-';
/// The byte used for padding.
const PADDING_CHARACTER: u8 = b' ';

/// The size of a simple timestamp.
const SIMPLE_LENGTH: usize = 15;
/// The padding required to fill the length of a simple timestamp.
const SIMPLE_PADDING: &[u8] = &[PADDING_CHARACTER; SIMPLE_LENGTH];
/// The format used to print simple timestamps.
const SIMPLE_FORMAT: &[BorrowedFormatItem<'static>] = time::macros::format_description!(
    version = 2,
    "[day padding:space] [month repr:short] '[year repr:last_two] [hour padding:space repr:24]:[minute padding:zero]"
);

/// The size of an ISO-8601 timestamp.
const ISO_8601_LENGTH: usize = 34;
/// The padding required to fill the length of an ISO-8601 timestamp.
const ISO_8601_PADDING: &[u8] = &[PADDING_CHARACTER; ISO_8601_LENGTH];
/// The format used to print simple ISO-8601 timestamps.
const ISO_8601_FORMAT: &Iso8601 = &Iso8601::DEFAULT;

thread_local! {
    /// Caches the system's offset to save repeated computation.
    static LOCAL_OFFSET: UtcOffset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
}

/// Determines how timestamps are displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Display in a simple format.
    Simple,
    /// Display in ISO-8601 format.
    Iso8601,
}

/// Determines what type of time section is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Created timestamp.
    Created,
    /// Accessed timestamp.
    Accessed,
    /// Modified timestamp.
    Modified,
}

/// A [`Section`] that writes an entry's extracted date.
#[derive(Clone, Copy, Debug)]
pub struct TimeSection {
    /// The time section type.
    pub kind: Kind,
    /// Determines how the date is rendered.
    pub format: Format,
}

impl Section for TimeSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, _: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let Some(timestamp_seconds) = entry.data.as_ref().map(|data| match self.kind {
            Kind::Created => data.ctime,
            Kind::Accessed => data.atime,
            Kind::Modified => data.mtime,
        }) else {
            let padding = if matches!(self.format, Format::Simple) { SIMPLE_PADDING } else { ISO_8601_PADDING };

            return if color {
                writev!(f, [&[MISSING_CHARACTER], padding] in BrightBlack)
            } else {
                writev!(f, [&[MISSING_CHARACTER], padding])
            };
        };

        let timestamp = LOCAL_OFFSET.with(|offset| {
            (OffsetDateTime::UNIX_EPOCH + SignedDuration::seconds(timestamp_seconds)).to_offset(*offset)
        });

        let Ok(formatted) = (if matches!(self.format, Format::Simple) {
            timestamp.format(SIMPLE_FORMAT)
        } else {
            timestamp.format(ISO_8601_FORMAT)
        }) else {
            unreachable!("timestamp format must be valid")
        };

        if !color {
            return writev!(f, [formatted.as_bytes()]);
        }

        match self.kind {
            Kind::Created => writev!(f, [formatted.as_bytes()] in BrightGreen),
            Kind::Accessed => writev!(f, [formatted.as_bytes()] in BrightCyan),
            Kind::Modified => writev!(f, [formatted.as_bytes()] in BrightBlue),
        }
    }
}
