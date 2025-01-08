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

//! Implements entry creation and modification date displays.

use std::fs::Metadata;
use std::io::{Result, StdoutLock};
use std::time::SystemTime;

use time::format_description::BorrowedFormatItem;
use time::format_description::well_known::{Iso8601, Rfc3339};
use time::{OffsetDateTime, UtcOffset};

use super::{Show, ShowData};
use crate::arguments::model::{Arguments, TimeVisibility};
use crate::{optionally_vector, optionally_vector_color};

/// The format used to print simple modification dates.
pub const SIMPLE_FORMAT: &[BorrowedFormatItem<'static>] = time::macros::format_description!(
    version = 2,
    "[day padding:space] [month repr:short] '[year repr:last_two] [hour padding:space repr:24]:[minute padding:zero]"
);

thread_local! {
    /// Caches the current offset.
    static OFFSET: UtcOffset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
}

/// Renders a file entry's modification date.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct Time {
    /// Determines whether to display modification dates.
    visibility: TimeVisibility,
    /// A function that extracts a date and time from metadata.
    extract: fn(&Metadata) -> Result<SystemTime>,
}

impl Time {
    /// Creates a new [`Modified`].
    pub fn new(visibility: TimeVisibility, extract: fn(&Metadata) -> Result<SystemTime>) -> Self {
        Self { visibility, extract }
    }
}

#[expect(clippy::expect_used, reason = "formatting only fails if the defined formats are somehow invalid")]
impl Show for Time {
    fn show_plain(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let Some(modified) = entry.data.and_then(|v| (self.extract)(v).ok()) else {
            return match self.visibility {
                TimeVisibility::Hide => unreachable!(),
                TimeVisibility::Simple => optionally_vector!(f, [b"-", &[b' '; 14]]),
                _ => optionally_vector!(f, [b"-", &[b' '; 34]]),
            };
        };

        let modified = OFFSET.with(|v| OffsetDateTime::from(modified).to_offset(*v));
        let string = match self.visibility {
            TimeVisibility::Hide => unreachable!(),
            TimeVisibility::Simple => modified.format(SIMPLE_FORMAT),
            TimeVisibility::Rfc3339 => modified.format(&Rfc3339),
            TimeVisibility::Iso8601 => modified.format(&Iso8601::DEFAULT),
        }
        .expect("will only fail if the formats are invalid");

        optionally_vector!(f, [string.as_bytes()])
    }

    fn show_color(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let Some(modified) = entry.data.and_then(|v| (self.extract)(v).ok()) else {
            return match self.visibility {
                TimeVisibility::Hide => unreachable!(),
                TimeVisibility::Simple => optionally_vector_color!(f, BrightBlack, [b"-", &[b' '; 14]]),
                _ => optionally_vector_color!(f, BrightBlack, [b"-", &[b' '; 34]]),
            };
        };

        let modified = OFFSET.with(|v| OffsetDateTime::from(modified).to_offset(*v));
        let string = match self.visibility {
            TimeVisibility::Hide => unreachable!(),
            TimeVisibility::Simple => modified.format(SIMPLE_FORMAT),
            TimeVisibility::Rfc3339 => modified.format(&Rfc3339),
            TimeVisibility::Iso8601 => modified.format(&Iso8601::DEFAULT),
        }
        .expect("will only fail if the formats are invalid");

        optionally_vector_color!(f, BrightBlue, [string.as_bytes()])
    }
}
