// SPDX-License-Identifier: GPL-3.0-or-later
//
// Copyright © 2025–2026 Jaxydog
//
// This file is part of fvr.
//
// fvr is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
//
// fvr is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with fvr. If not,
// see <https://www.gnu.org/licenses/>.

//! Implements a section that provides branches for tree-based views.

use std::io::{Result, StdoutLock};
use std::path::Path;

use recomposition::filter::Filter;

use super::Section;
use crate::files::{Entry, EntryMetadata};
use crate::writev;

/// The bytes used for padding.
const PADDING_CHARACTER: &[u8] = b" ";
/// The bytes used for a horizontal line.
const HORIZONTAL_CHARACTER: &[u8] = "─".as_bytes();
/// The bytes used for a horizontal split line.
const HORIZONTAL_BRANCH_CHARACTER: &[u8] = "┬".as_bytes();
/// The bytes used for a vertical line.
const VERTICAL_CHARACTER: &[u8] = "│".as_bytes();
/// The bytes used for a vertical split line.
const VERTICAL_BRANCH_CHARACTER: &[u8] = "├".as_bytes();
/// The bytes used for a top corner.
const VERTICAL_START_CHARACTER: &[u8] = "┌".as_bytes();
/// The bytes used for a bottom corner.
const VERTICAL_FINAL_CHARACTER: &[u8] = "└".as_bytes();

/// A [`Section`] that writes branches for tree-based views.
#[derive(Clone, Copy, Debug)]
pub struct TreeSection {
    /// The number of directories deep that should be displayed.
    pub max_depth: usize,
}

impl Section for TreeSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, parents: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let depth = parents.len();

        if entry.is_first() && depth == 0 {
            return if color {
                writev!(f, [VERTICAL_START_CHARACTER, HORIZONTAL_CHARACTER] in BrightBlack)
            } else {
                writev!(f, [VERTICAL_START_CHARACTER, HORIZONTAL_CHARACTER])
            };
        }

        let vertical_connection = if entry.is_last() { VERTICAL_FINAL_CHARACTER } else { VERTICAL_BRANCH_CHARACTER };
        let horizontal_connection = if depth < self.max_depth && entry.has_children() {
            HORIZONTAL_BRANCH_CHARACTER
        } else {
            HORIZONTAL_CHARACTER
        };

        let mut indent_buffer = Vec::with_capacity(parents.len() * 2);

        for parent_entry in parents.iter().skip(1) {
            if parent_entry.is_last() {
                indent_buffer.extend_from_slice(PADDING_CHARACTER);
            } else {
                indent_buffer.extend_from_slice(VERTICAL_CHARACTER);
            }

            indent_buffer.extend_from_slice(PADDING_CHARACTER);
        }

        if color {
            writev!(f, [
                &indent_buffer,
                vertical_connection,
                HORIZONTAL_CHARACTER,
                horizontal_connection,
                HORIZONTAL_CHARACTER
            ] in BrightBlack)
        } else {
            writev!(f, [
                &indent_buffer,
                vertical_connection,
                HORIZONTAL_CHARACTER,
                horizontal_connection,
                HORIZONTAL_CHARACTER
            ])
        }
    }
}
