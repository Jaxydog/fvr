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
use crate::arguments::model::TimeVisibility;
use crate::files::{Entry, EntryMetadata};
use crate::writev;

/// The byte used when the creation date cannot be determined.
pub const CHAR_MISSING: u8 = b'-';
/// The byte used for padding.
pub const CHAR_PADDING: u8 = b' ';
/// The size of a simple timestamp.
pub const SIZE_SIMPLE: usize = 15;
/// The size of an ISO-8601 timestamp.
pub const SIZE_ISO_8601: usize = 34;
/// The format used to print simple dates.
pub const SIMPLE_FORMAT: &[BorrowedFormatItem<'static>] = time::macros::format_description!(
    version = 2,
    "[day padding:space] [month repr:short] '[year repr:last_two] [hour padding:space repr:24]:[minute padding:zero]"
);

thread_local! {
    /// Caches the system's offset to save repeated computation.
    static OFFSET: UtcOffset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
}

/// Determines what type of time section is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeSectionType {
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
    /// Determines how the date is rendered.
    pub visibility: TimeVisibility,
    /// The time section type.
    pub kind: TimeSectionType,
}

impl TimeSection {
    /// Creates a new [`TimeSection`].
    #[inline]
    #[must_use]
    pub const fn new(visibility: TimeVisibility, kind: TimeSectionType) -> Self {
        Self { visibility, kind }
    }

    /// Creates a new [`TimeSection`] for a creation date timestamp.
    #[inline]
    #[must_use]
    pub const fn created(visibility: TimeVisibility) -> Self {
        Self::new(visibility, TimeSectionType::Created)
    }

    /// Creates a new [`TimeSection`] for an access date timestamp.
    #[inline]
    #[must_use]
    pub const fn accessed(visibility: TimeVisibility) -> Self {
        Self::new(visibility, TimeSectionType::Accessed)
    }

    /// Creates a new [`TimeSection`] for a modification date timestamp.
    #[inline]
    #[must_use]
    pub const fn modified(visibility: TimeVisibility) -> Self {
        Self::new(visibility, TimeSectionType::Modified)
    }
}

impl Section for TimeSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, _: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let Some(timestamp_seconds) = entry.data.as_ref().map(|data| match self.kind {
            TimeSectionType::Created => data.ctime,
            TimeSectionType::Accessed => data.atime,
            TimeSectionType::Modified => data.mtime,
        }) else {
            const PADDING_SIMPLE: &[u8] = &[CHAR_PADDING; SIZE_SIMPLE];
            const PADDING_ISO_8601: &[u8] = &[CHAR_PADDING; SIZE_ISO_8601];

            let padding = if self.visibility.is_simple() { PADDING_SIMPLE } else { PADDING_ISO_8601 };

            return if color {
                writev!(f, [&[CHAR_MISSING], padding] in BrightBlack)
            } else {
                writev!(f, [&[CHAR_MISSING], padding])
            };
        };

        let timestamp = OFFSET.with(|offset| {
            (OffsetDateTime::UNIX_EPOCH + SignedDuration::seconds(timestamp_seconds)).to_offset(*offset)
        });

        let Ok(formatted) = (if self.visibility.is_simple() {
            timestamp.format(SIMPLE_FORMAT)
        } else {
            timestamp.format(&Iso8601::DEFAULT)
        }) else {
            unreachable!("timestamp format must be valid")
        };

        if !color {
            return writev!(f, [formatted.as_bytes()]);
        }

        match self.kind {
            TimeSectionType::Created => writev!(f, [formatted.as_bytes()] in BrightGreen),
            TimeSectionType::Accessed => writev!(f, [formatted.as_bytes()] in BrightCyan),
            TimeSectionType::Modified => writev!(f, [formatted.as_bytes()] in BrightBlue),
        }
    }
}
