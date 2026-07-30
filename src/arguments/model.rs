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

//! Defines the command's argument data types.

use std::collections::HashSet;
use std::num::NonZero;
use std::path::Path;

use recomposition::sort::Sort;

use crate::files::EntryMetadata;
use crate::section::mode::ModeSection;
use crate::section::size::SizeSection;
use crate::section::time::TimeSection;
use crate::section::user::{GroupSection, UserSection};

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
    pub sort_order: Option<SortOrder>,
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
    /// Returns `true` if the color choice is [`Auto`].
    ///
    /// [`Auto`]: ColorChoice::Auto
    #[must_use]
    pub const fn is_auto(&self) -> bool {
        matches!(self, Self::Auto)
    }

    /// Returns `true` if the color choice is [`Always`].
    ///
    /// [`Always`]: ColorChoice::Always
    #[must_use]
    pub const fn is_always(&self) -> bool {
        matches!(self, Self::Always)
    }

    /// Returns `true` if the color choice is [`Never`].
    ///
    /// [`Never`]: ColorChoice::Never
    #[must_use]
    pub const fn is_never(&self) -> bool {
        matches!(self, Self::Never)
    }

    /// Returns whether or not color should be enabled.
    #[must_use]
    pub fn should_be_enabled(&self) -> bool {
        use supports_color::Stream;

        self.is_always() || (self.is_auto() && supports_color::on_cached(Stream::Stdout).is_some_and(|v| v.has_basic))
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
    pub user: Option<UserSection>,
    /// Whether to show owner groups.
    pub group: Option<GroupSection>,
}

/// The program's command-line arguments for the tree subcommand.
#[derive(Debug, Default)]
pub struct TreeArguments {
    /// The depth of the search.
    pub max_depth: Option<NonZero<usize>>,
}

/// Describes how entries should be sorted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SortOrder {
    /// Alphabetically.
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
    Directories,
    /// Files.
    Files,
    /// Symbolic links.
    Symlinks,
    /// Reversed order.
    Reverse(Box<Self>),
    /// Chained order, preferring the left-most order.
    Then(Box<(Self, Self)>),
}

impl SortOrder {
    /// Chains this order with another, preferring this ordering.
    #[inline]
    #[must_use]
    pub fn then(self, other: Self) -> Self {
        Self::Then(Box::new((self, other)))
    }

    /// Reverses the ordering of this sort.
    #[inline]
    #[must_use]
    pub fn reverse(self) -> Self {
        match self {
            Self::Reverse(sort) => *sort,
            sort => Self::Reverse(Box::new(sort)),
        }
    }

    /// Returns a reference to the most recent [`SortOrder`].
    #[must_use]
    pub fn top(&self) -> &Self {
        match self {
            Self::Then(v) => v.1.top(),
            _ => self,
        }
    }
}

impl Sort<(Box<Path>, EntryMetadata)> for SortOrder {
    fn compare(&self, lhs: &(Box<Path>, EntryMetadata), rhs: &(Box<Path>, EntryMetadata)) -> std::cmp::Ordering {
        use recomposition::sort::{order, partial_order};

        match self {
            Self::Name => order().map_ref(Path::as_os_str).compare(&lhs.0, &rhs.0),
            Self::Accessed => partial_order().reverse().map(|m: &EntryMetadata| m.atime).compare(&lhs.1, &rhs.1),
            Self::Created => partial_order().reverse().map(|m: &EntryMetadata| m.ctime).compare(&lhs.1, &rhs.1),
            Self::Modified => partial_order().reverse().map(|m: &EntryMetadata| m.mtime).compare(&lhs.1, &rhs.1),
            Self::Size => order().map(|m: &EntryMetadata| m.size).compare(&lhs.1, &rhs.1),
            Self::Hidden => order().reverse().map(|p| crate::files::is_hidden(p)).compare(&lhs.0, &rhs.0),
            Self::Directories => {
                order().reverse().map(|m: &EntryMetadata| m.filetype().is_directory()).compare(&lhs.1, &rhs.1)
            }
            Self::Files => order().reverse().map(|m: &EntryMetadata| m.filetype().is_file()).compare(&lhs.1, &rhs.1),
            Self::Symlinks => {
                order().reverse().map(|m: &EntryMetadata| m.filetype().is_symbolic_link()).compare(&lhs.1, &rhs.1)
            }
            Self::Reverse(sort_order) => sort_order.reverse().compare(lhs, rhs),
            Self::Then(orders) => (&orders.0).then(&orders.1).compare(lhs, rhs),
        }
    }
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Directories.then(Self::Files).then(Self::Name)
    }
}

/// Determines whether to display file sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeVisibility {
    /// Files sizes are not rendered.
    #[default]
    Hide,
    /// Output the number of bytes.
    Simple,
    /// Output the size in base 2.
    Base2,
    /// Output the size in base 10.
    Base10,
}

impl SizeVisibility {
    /// Returns `true` if the size visibility is [`Hide`].
    ///
    /// [`Hide`]: SizeVisibility::Hide
    #[must_use]
    pub const fn is_hide(&self) -> bool {
        matches!(self, Self::Hide)
    }

    /// Returns `true` if the size visibility is [`Simple`].
    ///
    /// [`Simple`]: SizeVisibility::Simple
    #[must_use]
    pub const fn is_simple(&self) -> bool {
        matches!(self, Self::Simple)
    }

    /// Returns `true` if the size visibility is [`Base2`].
    ///
    /// [`Base2`]: SizeVisibility::Base2
    #[must_use]
    pub const fn is_base2(&self) -> bool {
        matches!(self, Self::Base2)
    }

    /// Returns `true` if the size visibility is [`Base10`].
    ///
    /// [`Base10`]: SizeVisibility::Base10
    #[must_use]
    pub const fn is_base10(&self) -> bool {
        matches!(self, Self::Base10)
    }
}

/// Determines whether to display dates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimeVisibility {
    /// Dates are not rendered.
    #[default]
    Hide,
    /// Display in a simple format.
    Simple,
    /// Display in ISO-8601 format.
    Iso8601,
}

impl TimeVisibility {
    /// Returns `true` if the time visibility is [`Hide`].
    ///
    /// [`Hide`]: TimeVisibility::Hide
    #[must_use]
    pub const fn is_hide(&self) -> bool {
        matches!(self, Self::Hide)
    }

    /// Returns `true` if the time visibility is [`Simple`].
    ///
    /// [`Simple`]: TimeVisibility::Simple
    #[must_use]
    pub const fn is_simple(&self) -> bool {
        matches!(self, Self::Simple)
    }

    /// Returns `true` if the time visibility is [`Iso8601`].
    ///
    /// [`Iso8601`]: TimeVisibility::Iso8601
    #[must_use]
    pub const fn is_iso8601(&self) -> bool {
        matches!(self, Self::Iso8601)
    }
}
