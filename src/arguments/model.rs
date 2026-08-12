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

//! Defines the command's argument data types.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::num::NonZero;
use std::path::Path;

use recomposition::sort::Sort;

use crate::files::EntryMetadata;
use crate::section::mode::ModeSection;
use crate::section::owner::OwnerSection;
use crate::section::size::SizeSection;
use crate::section::time::TimeSection;

/// The program's command-line arguments.
#[derive(Debug, Default)]
pub struct Arguments {
    /// The paths to list.
    pub paths: Vec<Box<Path>>,
    /// Determines whether to output using color.
    pub color: ColorChoice,
    /// Whether to show hidden files.
    pub show_hidden: bool,
    /// Whether to resolve symbolic links.
    pub resolve_symlinks: bool,
    /// The paths to exclude.
    pub excluded: Option<HashSet<Box<Path>>>,
    /// The paths to include.
    pub included: Option<HashSet<Box<Path>>>,
    /// The preferred sorting order.
    pub sort_order: SortOrder,
    /// The program's selected subcommand.
    pub command: Option<SubCommand>,
}

/// Determines whether to output using color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorChoice {
    /// Automatically determine whether to output with color.
    #[default]
    Auto,
    /// Always output with color.
    Always,
    /// Never output with color.
    Never,
}

impl ColorChoice {
    /// Returns whether or not color should be enabled.
    #[must_use]
    pub fn should_be_enabled(self) -> bool {
        use supports_color::Stream::Stdout;
        use supports_color::on_cached;

        matches!(self, Self::Always) || (matches!(self, Self::Auto) && on_cached(Stdout).is_some_and(|v| v.has_basic))
    }
}

/// Implements a sort function based on a list of [`EntrySortOrder`] variants.
#[derive(Clone, Debug)]
pub struct SortOrder {
    /// The inner sort order.
    inner: Vec<(SortOrderType, bool)>,
}

impl SortOrder {
    /// Clears the inner sort order.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Adds a sort order.
    pub fn add(&mut self, order: SortOrderType, reverse: bool) {
        self.inner.push((order, reverse));
    }
}

impl Default for SortOrder {
    fn default() -> Self {
        Self {
            inner: vec![(SortOrderType::Directory, false), (SortOrderType::File, false), (SortOrderType::Name, false)],
        }
    }
}

impl Sort<(Box<Path>, EntryMetadata)> for SortOrder {
    fn compare(&self, lhs: &(Box<Path>, EntryMetadata), rhs: &(Box<Path>, EntryMetadata)) -> std::cmp::Ordering {
        self.inner.iter().copied().fold(Ordering::Equal, |ordering, (variant, reverse)| {
            ordering.then_with(|| {
                let next = variant.compare(lhs, rhs);

                if reverse { next.reverse() } else { next }
            })
        })
    }
}

/// Defines a possible sort order for displayed entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortOrderType {
    /// Filename.
    Name,
    /// Access date.
    Accessed,
    /// Creation date.
    Created,
    /// Modification date.
    Modified,
    /// File size.
    Size,
    /// Hidden files.
    Hidden,
    /// Directories.
    Directory,
    /// Files.
    File,
    /// Symbolic links.
    SymbolicLink,
}

impl Sort<(Box<Path>, EntryMetadata)> for SortOrderType {
    fn compare(&self, lhs: &(Box<Path>, EntryMetadata), rhs: &(Box<Path>, EntryMetadata)) -> std::cmp::Ordering {
        match self {
            Self::Name => recomposition::sort::order().map_ref(Path::as_os_str).compare(&lhs.0, &rhs.0),
            Self::Accessed => {
                recomposition::sort::order().reverse().map(|data: &EntryMetadata| data.atime).compare(&lhs.1, &rhs.1)
            }
            Self::Created => {
                recomposition::sort::order().reverse().map(|data: &EntryMetadata| data.ctime).compare(&lhs.1, &rhs.1)
            }
            Self::Modified => {
                recomposition::sort::order().reverse().map(|data: &EntryMetadata| data.mtime).compare(&lhs.1, &rhs.1)
            }
            Self::Size => {
                recomposition::sort::order().map(|data: &EntryMetadata| data.size).compare(&lhs.1, &rhs.1) //
            }
            Self::Hidden => recomposition::sort::order()
                .reverse()
                .map(|path: &Path| crate::files::is_hidden(path))
                .compare(&lhs.0, &rhs.0),
            Self::Directory => recomposition::sort::order()
                .reverse()
                .map(|data: &EntryMetadata| data.filetype().is_directory())
                .compare(&lhs.1, &rhs.1),
            Self::File => recomposition::sort::order()
                .reverse()
                .map(|data: &EntryMetadata| data.filetype().is_file())
                .compare(&lhs.1, &rhs.1),
            Self::SymbolicLink => recomposition::sort::order()
                .reverse()
                .map(|data: &EntryMetadata| data.filetype().is_symbolic_link())
                .compare(&lhs.1, &rhs.1),
        }
    }
}

/// The program's subcommand.
#[derive(Debug)]
pub enum SubCommand {
    /// The list subcommand.
    List(ListArguments),
    /// The tree subcommand.
    Tree(TreeArguments),
}

impl SubCommand {
    /// Returns `true` if the subcommand is [`List`].
    ///
    /// [`List`]: SubCommand::List
    #[must_use]
    pub const fn is_list(&self) -> bool {
        matches!(self, Self::List(..))
    }

    /// Returns `true` if the subcommand is [`Tree`].
    ///
    /// [`Tree`]: SubCommand::Tree
    #[must_use]
    pub const fn is_tree(&self) -> bool {
        matches!(self, Self::Tree(..))
    }
}

/// The program's command-line arguments for the list subcommand.
#[derive(Debug, Default)]
pub struct ListArguments {
    /// The preferred mode visibility.
    pub mode: Option<ModeSection>,
    /// The preferred size visibility.
    pub size: Option<SizeSection>,
    /// The preferred creation date visibility.
    pub created: Option<TimeSection>,
    /// The preferred access date visibility.
    pub accessed: Option<TimeSection>,
    /// The preferred modification date visibility.
    pub modified: Option<TimeSection>,
    /// Whether to show owner users.
    pub user: Option<OwnerSection>,
    /// Whether to show owner groups.
    pub group: Option<OwnerSection>,
}

/// The program's command-line arguments for the tree subcommand.
#[derive(Debug, Default)]
pub struct TreeArguments {
    /// The depth of the search.
    pub max_depth: Option<NonZero<usize>>,
}
