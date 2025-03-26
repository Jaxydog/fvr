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

//! Defines the command's argument data types.

use std::collections::HashSet;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use recomposition::sort::Sort;

/// The program's command-line arguments.
#[derive(Default)]
pub struct Arguments {
    /// Determines whether to output using color.
    pub color: ColorChoice,
    /// The program's selected sub-command.
    pub command: Option<SubCommand>,
}

impl Arguments {
    /// Returns the current schema of this [`Arguments`].
    ///
    /// # Panics
    ///
    /// Panics if the current schema has not been defined.
    #[expect(clippy::expect_used, reason = "we cannot return a schema for a sub-command if it has not been defined")]
    pub const fn current_schema(&self) -> super::schema::Command<'static> {
        #[inline]
        const fn sub_schema(index: usize) -> super::schema::Command<'static> {
            let list = super::SCHEMA.sub_commands.expect("no sub-commands have been defined");

            assert!(index < list.len(), "missing required sub-command definition");

            list[index]
        }

        match self.command {
            None => super::SCHEMA,
            Some(SubCommand::List(..)) => sub_schema(0),
            Some(SubCommand::Tree(..)) => sub_schema(1),
        }
    }
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
}

/// The program's sub-command.
pub enum SubCommand {
    /// The list sub-command.
    List(ListArguments),
    /// The tree sub-command.
    Tree(TreeArguments),
}

impl SubCommand {
    /// Returns `true` if the sub-command is [`List`].
    ///
    /// [`List`]: SubCommand::List
    #[must_use]
    pub const fn is_list(&self) -> bool {
        matches!(self, Self::List(..))
    }

    /// Returns `true` if the sub-command is [`Tree`].
    ///
    /// [`Tree`]: SubCommand::Tree
    #[must_use]
    pub const fn is_tree(&self) -> bool {
        matches!(self, Self::Tree(..))
    }

    /// Returns the inner value of this sub-command if it is a [`List`].
    ///
    /// [`List`]: SubCommand::List
    #[must_use]
    pub const fn as_list(&self) -> Option<&ListArguments> {
        if let Self::List(v) = self { Some(v) } else { None }
    }

    /// Returns the inner value of this sub-command if it is a [`Tree`].
    ///
    /// [`Tree`]: SubCommand::Tree
    #[must_use]
    pub const fn as_tree(&self) -> Option<&TreeArguments> {
        if let Self::Tree(v) = self { Some(v) } else { None }
    }
}

/// The program's command-line arguments for the list sub-command.
#[derive(Default)]
#[expect(clippy::struct_excessive_bools, reason = "such is the nature of command-line flags")]
pub struct ListArguments {
    /// The paths to list.
    pub paths: Paths,
    /// Whether to show hidden files.
    pub show_hidden: bool,
    /// Whether to resolve symbolic links.
    pub resolve_symlinks: bool,
    /// The preferred sorting function.
    pub sorting: Option<SortOrder>,
    /// The preferred mode visibility.
    pub mode: ModeVisibility,
    /// The preferred size visibility.
    pub size: SizeVisibility,
    /// The preferred creation date visibility.
    pub created: TimeVisibility,
    /// The preferred access date visibility.
    pub accessed: TimeVisibility,
    /// The preferred modification date visibility.
    pub modified: TimeVisibility,
    /// Whether to show owner users.
    pub user: bool,
    /// Whether to show owner groups.
    pub group: bool,
    /// The paths to exclude.
    pub excluded: Option<Paths>,
    /// The paths to include.
    pub included: Option<Paths>,
}

/// The program's command-line arguments for the tree sub-command.
#[derive(Default)]
pub struct TreeArguments {
    /// The paths to list.
    pub paths: Paths,
    /// Whether to show hidden files.
    pub show_hidden: bool,
    /// Whether to resolve symbolic links.
    pub resolve_symlinks: bool,
    /// The preferred sorting function.
    pub sorting: Option<SortOrder>,
    /// The paths to exclude.
    pub excluded: Option<Paths>,
    /// The paths to include.
    pub included: Option<Paths>,
}

/// The paths to list.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paths {
    /// The inner set of paths.
    inner: HashSet<Box<Path>>,
}

impl Paths {
    /// Creates a new [`Paths`].
    #[must_use]
    pub fn new() -> Self {
        Self { inner: HashSet::new() }
    }

    /// Returns `true` if this value contains the given path.
    pub fn has(&self, path: impl AsRef<Path>) -> bool {
        self.inner.contains(path.as_ref())
    }

    /// Adds the given path to the list.
    pub fn add(&mut self, path: impl AsRef<Path>) {
        self.inner.insert(Box::from(path.as_ref()));
    }

    /// Returns an iterator of the inner paths.
    pub fn get(&self) -> impl Iterator<Item = &Path> {
        self.inner.iter().map(|v| &(**v))
    }

    /// Returns the number of paths.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if no paths have been added.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
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

impl Sort<(PathBuf, Metadata)> for SortOrder {
    fn compare(&self, lhs: &(PathBuf, Metadata), rhs: &(PathBuf, Metadata)) -> std::cmp::Ordering {
        use recomposition::sort::{order, partial_order};

        match self {
            Self::Name => order().map_ref(Path::as_os_str).compare(&lhs.0, &rhs.0),
            Self::Accessed => partial_order().reverse().map(|m: &Metadata| m.accessed().ok()).compare(&lhs.1, &rhs.1),
            Self::Created => partial_order().reverse().map(|m: &Metadata| m.created().ok()).compare(&lhs.1, &rhs.1),
            Self::Modified => partial_order().reverse().map(|m: &Metadata| m.modified().ok()).compare(&lhs.1, &rhs.1),
            Self::Size => order().map(MetadataExt::size).compare(&lhs.1, &rhs.1),
            Self::Hidden => order().reverse().map(|p| crate::files::is_hidden(p)).compare(&lhs.0, &rhs.0),
            Self::Directories => order().reverse().map(Metadata::is_dir).compare(&lhs.1, &rhs.1),
            Self::Files => order().reverse().map(Metadata::is_file).compare(&lhs.1, &rhs.1),
            Self::Symlinks => order().reverse().map(Metadata::is_symlink).compare(&lhs.1, &rhs.1),
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

/// Determines whether to display an entry's Unix file mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModeVisibility {
    /// Do not show entry modes.
    #[default]
    Hide,
    /// Show standard entry modes.
    Show,
    /// Show extended entry modes.
    Extended,
}

impl ModeVisibility {
    /// Returns `true` if the mode visibility is [`Hide`].
    ///
    /// [`Hide`]: ModeVisibility::Hide
    #[must_use]
    pub const fn is_hide(&self) -> bool {
        matches!(self, Self::Hide)
    }

    /// Returns `true` if the mode visibility is [`Show`].
    ///
    /// [`Show`]: ModeVisibility::Show
    #[must_use]
    pub const fn is_show(&self) -> bool {
        matches!(self, Self::Show)
    }

    /// Returns `true` if the mode visibility is [`Extended`].
    ///
    /// [`Extended`]: ModeVisibility::Extended
    #[must_use]
    pub const fn is_extended(&self) -> bool {
        matches!(self, Self::Extended)
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
