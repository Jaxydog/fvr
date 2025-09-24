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

//! Implements the tree sub-command.

use std::io::Write;
use std::num::NonZero;
use std::rc::Rc;

use crate::arguments::model::{Arguments, SubCommand};
use crate::files::{Entry, is_hidden};
use crate::section::Section;
use crate::section::name::NameSection;
use crate::section::tree::TreeSection;

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(arguments: &Arguments) -> std::io::Result<()> {
    let Some(SubCommand::Tree(tree_arguments)) = arguments.command.as_ref() else { unreachable!() };

    let sort = tree_arguments.sorting.clone().unwrap_or_default();
    let filter = recomposition::filter::from_fn(|(path, _)| {
        (tree_arguments.show_hidden || !is_hidden(path))
            && tree_arguments.included.as_ref().is_none_or(|include| include.has(path))
            && !tree_arguments.excluded.as_ref().is_some_and(|exclude| exclude.has(path))
    });

    let tree_section = TreeSection::new(tree_arguments.max_depth.map_or(usize::MAX, NonZero::get));
    let name_section = NameSection::new(true, tree_arguments.resolve_symlinks);

    let f = &mut std::io::stdout().lock();

    for (index, path) in tree_arguments.paths.get().enumerate() {
        let data = std::fs::symlink_metadata(path).ok();
        let entry = Rc::new(Entry::root(path, data.as_ref(), &filter));

        if index > 0 {
            f.write_all(b"\n")?;
        }

        tree_section.write(arguments.color, f, &[], &entry)?;
        NameSection::new(true, false).write(arguments.color, f, &[], &entry)?;

        f.write_all(b"\n")?;

        crate::files::visit_entries_recursive(
            &entry,
            tree_arguments.max_depth,
            &filter,
            &sort,
            &mut |parents, entry| {
                tree_section.write(arguments.color, f, parents, &entry)?;
                name_section.write(arguments.color, f, parents, &entry)?;

                f.write_all(b"\n")
            },
        )?;
    }

    f.flush()
}
