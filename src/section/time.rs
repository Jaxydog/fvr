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

//! Implements sections related to entry timestamps.

use std::io::{Result, Write};
use std::rc::Rc;

use time::format_description::BorrowedFormatItem;
use time::format_description::well_known::Iso8601;
use time::{OffsetDateTime, UtcOffset};

use super::Section;
use crate::arguments::model::TimeVisibility;
use crate::files::Entry;
use crate::files::filter::Filter;
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
    #[must_use]
    pub const fn new(visibility: TimeVisibility, kind: TimeSectionType) -> Self {
        Self { visibility, kind }
    }

    /// Creates a new [`TimeSection`] for a creation date timestamp.
    #[must_use]
    pub const fn created(visibility: TimeVisibility) -> Self {
        Self::new(visibility, TimeSectionType::Created)
    }

    /// Creates a new [`TimeSection`] for an access date timestamp.
    #[must_use]
    pub const fn accessed(visibility: TimeVisibility) -> Self {
        Self::new(visibility, TimeSectionType::Accessed)
    }

    /// Creates a new [`TimeSection`] for a modification date timestamp.
    #[must_use]
    pub const fn modified(visibility: TimeVisibility) -> Self {
        Self::new(visibility, TimeSectionType::Modified)
    }
}

#[expect(clippy::expect_used, reason = "formatting only fails if the defined formats are somehow invalid")]
impl Section for TimeSection {
    fn write_plain<W, F>(&self, f: &mut W, _: &[&Rc<Entry<F>>], entry: &Rc<Entry<F>>) -> Result<()>
    where
        W: Write,
        F: Filter,
    {
        let Some(timestamp) = entry.data.and_then(|v| match self.kind {
            TimeSectionType::Created => v.created().ok(),
            TimeSectionType::Accessed => v.accessed().ok(),
            TimeSectionType::Modified => v.modified().ok(),
        }) else {
            return writev!(f, [
                &[CHAR_MISSING],
                if self.visibility.is_simple() { &[CHAR_PADDING; SIZE_SIMPLE] } else { &[CHAR_PADDING; SIZE_ISO_8601] }
            ]);
        };

        let timestamp = OFFSET.with(|v| OffsetDateTime::from(timestamp).to_offset(*v));
        let formatted = match self.visibility {
            TimeVisibility::Simple => timestamp.format(SIMPLE_FORMAT),
            TimeVisibility::Iso8601 => timestamp.format(&Iso8601::DEFAULT),
            TimeVisibility::Hide => unreachable!(),
        }
        .expect("will only fail if the formats are invalid");

        writev!(f, [formatted.as_bytes()])
    }

    fn write_color<W, F>(&self, f: &mut W, _: &[&Rc<Entry<F>>], entry: &Rc<Entry<F>>) -> Result<()>
    where
        W: Write,
        F: Filter,
    {
        let Some(timestamp) = entry.data.and_then(|v| match self.kind {
            TimeSectionType::Created => v.created().ok(),
            TimeSectionType::Accessed => v.accessed().ok(),
            TimeSectionType::Modified => v.modified().ok(),
        }) else {
            return writev!(f, [
                &[CHAR_MISSING],
                if self.visibility.is_simple() { &[CHAR_PADDING; SIZE_SIMPLE] } else { &[CHAR_PADDING; SIZE_ISO_8601] }
            ] in BrightBlack);
        };

        let timestamp = OFFSET.with(|v| OffsetDateTime::from(timestamp).to_offset(*v));
        let formatted = match self.visibility {
            TimeVisibility::Simple => timestamp.format(SIMPLE_FORMAT),
            TimeVisibility::Iso8601 => timestamp.format(&Iso8601::DEFAULT),
            TimeVisibility::Hide => unreachable!(),
        }
        .expect("will only fail if the formats are invalid");

        match self.kind {
            TimeSectionType::Created => writev!(f, [formatted.as_bytes()] in BrightGreen),
            TimeSectionType::Accessed => writev!(f, [formatted.as_bytes()] in BrightCyan),
            TimeSectionType::Modified => writev!(f, [formatted.as_bytes()] in BrightBlue),
        }
    }
}
