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
use time::format_description::well_known::{Iso8601, Rfc3339};
use time::{OffsetDateTime, UtcOffset};

use super::Section;
use crate::arguments::model::TimeVisibility;
use crate::files::Entry;
use crate::writev;

/// The byte used when the creation date cannot be determined.
pub const CHAR_MISSING: u8 = b'-';
/// The byte used for padding.
pub const CHAR_PADDING: u8 = b' ';
/// The format used to print simple dates.
pub const SIMPLE_FORMAT: &[BorrowedFormatItem<'static>] = time::macros::format_description!(
    version = 2,
    "[day padding:space] [month repr:short] '[year repr:last_two] [hour padding:space repr:24]:[minute padding:zero]"
);

thread_local! {
    /// Caches the system's offset to save repeated computation.
    static OFFSET: UtcOffset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
}

/// A [`Section`] that writes an entry's creation date.
#[derive(Clone, Copy, Debug)]
pub struct CreatedSection {
    /// Determines how the date is rendered.
    pub visibility: TimeVisibility,
}

#[expect(clippy::expect_used, reason = "formatting only fails if the defined formats are somehow invalid")]
impl Section for CreatedSection {
    fn write_plain<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let Some(created) = entry.data.and_then(|v| v.created().ok()) else {
            return writev!(f, [
                &[CHAR_MISSING],
                if self.visibility.is_simple() { &[CHAR_PADDING; 14] } else { &[CHAR_PADDING; 34] }
            ]);
        };

        let created = OFFSET.with(|v| OffsetDateTime::from(created).to_offset(*v));
        let formatted = match self.visibility {
            TimeVisibility::Hide => unreachable!(),
            TimeVisibility::Simple => created.format(SIMPLE_FORMAT),
            TimeVisibility::Rfc3339 => created.format(&Rfc3339),
            TimeVisibility::Iso8601 => created.format(&Iso8601::DEFAULT),
        }
        .expect("will only fail if the formats are invalid");

        writev!(f, [formatted.as_bytes()])
    }

    fn write_color<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let Some(modified) = entry.data.and_then(|v| v.created().ok()) else {
            return writev!(f, [
                &[CHAR_MISSING],
                if self.visibility.is_simple() { &[CHAR_PADDING; 14] } else { &[CHAR_PADDING; 34] }
            ] in BrightBlack);
        };

        let modified = OFFSET.with(|v| OffsetDateTime::from(modified).to_offset(*v));
        let formatted = match self.visibility {
            TimeVisibility::Hide => unreachable!(),
            TimeVisibility::Simple => modified.format(SIMPLE_FORMAT),
            TimeVisibility::Rfc3339 => modified.format(&Rfc3339),
            TimeVisibility::Iso8601 => modified.format(&Iso8601::DEFAULT),
        }
        .expect("will only fail if the formats are invalid");

        writev!(f, [formatted.as_bytes()] in BrightCyan)
    }
}

/// A [`Section`] that writes an entry's creation date.
#[derive(Clone, Copy, Debug)]
pub struct ModifiedSection {
    /// Determines how the date is rendered.
    pub visibility: TimeVisibility,
}

#[expect(clippy::expect_used, reason = "formatting only fails if the defined formats are somehow invalid")]
impl Section for ModifiedSection {
    fn write_plain<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let Some(modified) = entry.data.and_then(|v| v.modified().ok()) else {
            return writev!(f, [
                &[CHAR_MISSING],
                if self.visibility.is_simple() { &[CHAR_PADDING; 14] } else { &[CHAR_PADDING; 34] }
            ]);
        };

        let modified = OFFSET.with(|v| OffsetDateTime::from(modified).to_offset(*v));
        let formatted = match self.visibility {
            TimeVisibility::Hide => unreachable!(),
            TimeVisibility::Simple => modified.format(SIMPLE_FORMAT),
            TimeVisibility::Rfc3339 => modified.format(&Rfc3339),
            TimeVisibility::Iso8601 => modified.format(&Iso8601::DEFAULT),
        }
        .expect("will only fail if the formats are invalid");

        writev!(f, [formatted.as_bytes()])
    }

    fn write_color<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let Some(modified) = entry.data.and_then(|v| v.modified().ok()) else {
            return writev!(f, [
                &[CHAR_MISSING],
                if self.visibility.is_simple() { &[CHAR_PADDING; 14] } else { &[CHAR_PADDING; 34] }
            ] in BrightBlack);
        };

        let modified = OFFSET.with(|v| OffsetDateTime::from(modified).to_offset(*v));
        let formatted = match self.visibility {
            TimeVisibility::Hide => unreachable!(),
            TimeVisibility::Simple => modified.format(SIMPLE_FORMAT),
            TimeVisibility::Rfc3339 => modified.format(&Rfc3339),
            TimeVisibility::Iso8601 => modified.format(&Iso8601::DEFAULT),
        }
        .expect("will only fail if the formats are invalid");

        writev!(f, [formatted.as_bytes()] in BrightBlue)
    }
}
