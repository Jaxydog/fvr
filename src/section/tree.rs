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

//! Implements a section that provides branches for tree-based views.

use std::io::{Result, Write};
use std::rc::Rc;

use super::Section;
use crate::files::Entry;
use crate::writev;

/// A [`Section`] that writes branches for tree-based views.
#[derive(Clone, Copy, Debug)]
pub struct TreeSection;

impl TreeSection {
    /// The bytes used for a bottom corner.
    pub const CORNER_BOTTOM: &[u8] = "└".as_bytes();
    /// The bytes used for a top corner.
    pub const CORNER_TOP: &[u8] = "┌".as_bytes();
    /// The bytes used for a horizontal line.
    pub const LINE_HORIZONTAL: &[u8] = "─".as_bytes();
    /// The bytes used for a vertical line.
    pub const LINE_VERTICAL: &[u8] = "│".as_bytes();
    /// The bytes used for padding.
    pub const PADDING: &[u8] = b" ";
    /// The bytes used for a horizontal split line.
    pub const SPLIT_HORIZONTAL: &[u8] = "┬".as_bytes();
    /// The bytes used for a vertical split line.
    pub const SPLIT_VERTICAL: &[u8] = "├".as_bytes();
}

impl Section for TreeSection {
    fn write_plain<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let depth = parents.len();

        if entry.is_first() && depth == 0 {
            return writev!(f, [Self::CORNER_TOP, Self::LINE_HORIZONTAL]);
        }

        let join = if entry.is_last() { Self::CORNER_BOTTOM } else { Self::SPLIT_VERTICAL };
        let connect = if entry.has_children() { Self::SPLIT_HORIZONTAL } else { Self::LINE_HORIZONTAL };

        let mut buffer = Vec::with_capacity(parents.len() * 2);

        for parent in parents.iter().skip(1) {
            if parent.is_last() {
                buffer.extend_from_slice(Self::PADDING);
            } else {
                buffer.extend_from_slice(Self::LINE_VERTICAL);
            }

            buffer.extend_from_slice(Self::PADDING);
        }

        writev!(f, [&buffer, join, Self::LINE_HORIZONTAL, connect, Self::LINE_HORIZONTAL])
    }

    fn write_color<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let depth = parents.len();

        if entry.is_first() && depth == 0 {
            return writev!(f, [Self::CORNER_TOP, Self::LINE_HORIZONTAL] in BrightBlack);
        }

        let join = if entry.is_last() { Self::CORNER_BOTTOM } else { Self::SPLIT_VERTICAL };
        let connect = if entry.has_children() { Self::SPLIT_HORIZONTAL } else { Self::LINE_HORIZONTAL };

        let mut buffer = Vec::with_capacity(parents.len() * 2);

        for parent in parents.iter().skip(1) {
            if parent.is_last() {
                buffer.extend_from_slice(Self::PADDING);
            } else {
                buffer.extend_from_slice(Self::LINE_VERTICAL);
            }

            buffer.extend_from_slice(Self::PADDING);
        }

        writev!(f, [&buffer, join, Self::LINE_HORIZONTAL, connect, Self::LINE_HORIZONTAL] in BrightBlack)
    }
}
