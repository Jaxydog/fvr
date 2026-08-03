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

//! Implements the tree subcommand.

use std::io::Write;
use std::num::NonZero;
use std::path::Path;

use recomposition::sort::ListSortExt;

use crate::arguments::model::{Arguments, SubCommand};
use crate::files::{Entry, EntryMetadata};
use crate::section::Section;
use crate::section::name::NameSection;
use crate::section::tree::TreeSection;

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(arguments: Arguments) -> std::io::Result<()> {
    let Some(SubCommand::Tree(tree_arguments)) = arguments.command else { unreachable!() };

    let tree_section = TreeSection { max_depth: tree_arguments.max_depth.map_or(usize::MAX, NonZero::get) };
    let name_section = NameSection { trim_paths: true, resolve_symlinks: arguments.resolve_symlinks };

    let filter = recomposition::filter::from_fn(|(path, _)| {
        (arguments.show_hidden || !crate::files::is_hidden(path))
            && arguments.included.as_ref().is_none_or(|include| include.contains(path))
            && !arguments.excluded.as_ref().is_some_and(|exclude| exclude.contains(path))
    });

    let paths = arguments.paths.into_iter().map(|path| {
        let data = std::fs::symlink_metadata(&path)?;

        Ok((path, EntryMetadata::new(&data)))
    });

    let mut paths = paths.collect::<std::io::Result<Box<[(Box<Path>, EntryMetadata)]>>>()?;

    paths.sort_unstable_with(&arguments.sort_order);

    let should_use_color = arguments.color.should_be_enabled();
    let f = &mut std::io::stdout().lock();

    for (index, (path, data)) in paths.into_iter().enumerate() {
        let entry = Entry::root(path, Some(data), &filter);

        if index > 0 {
            f.write_all(b"\n")?;
        }

        if entry.can_traverse() {
            tree_section.write(should_use_color, f, &[], &entry)?;
            name_section.write(should_use_color, f, &[], &entry)?;
        } else {
            let path = entry.path.absolute()?.parent().map_or_else(|| Path::new("/").into(), Box::from);
            let entry = Entry::root(path, None, &filter);

            tree_section.write(should_use_color, f, &[], &entry)?;
            name_section.write(should_use_color, f, &[], &entry)?;
        }

        f.write_all(b"\n")?;

        crate::files::visit_entries_recursive(
            &entry,
            tree_arguments.max_depth,
            &filter,
            &arguments.sort_order,
            &mut |parents, entry| {
                tree_section.write(should_use_color, f, parents, entry)?;
                name_section.write(should_use_color, f, parents, entry)?;

                f.write_all(b"\n")
            },
        )?;
    }

    f.flush()
}
