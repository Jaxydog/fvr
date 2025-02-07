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
use std::rc::Rc;

use crate::arguments::model::{Arguments, SubCommand, TreeArguments};
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
    let TreeArguments { paths, show_hidden, resolve_symlinks, sorting, ignored } = tree_arguments;

    let sort = sorting.clone().unwrap_or_default();
    let sort = sort.compile();
    let filter = crate::files::filter::by(|v, _| {
        // Check for hidden files, then exclude any ignored files.
        (*show_hidden || !is_hidden(v)) && !ignored.as_ref().is_some_and(|i| i.has(v))
    });

    let name_section = NameSection::new(true, *resolve_symlinks);

    let f = &mut std::io::stdout().lock();

    for (index, path) in paths.get().enumerate() {
        let data = std::fs::symlink_metadata(path).ok();
        let entry = Rc::new(Entry::root(path, data.as_ref()));

        if index > 0 {
            f.write_all(b"\n")?;
        }

        TreeSection.write(arguments.color, f, &[], &entry)?;
        NameSection::new(true, false).write(arguments.color, f, &[], &entry)?;

        f.write_all(b"\n")?;

        crate::files::visit_entries_recursive(&entry, &filter, &sort, &mut |parents, entry| {
            TreeSection.write(arguments.color, f, parents, &entry)?;

            name_section.write(arguments.color, f, parents, &entry)?;

            f.write_all(b"\n")
        })?;
    }

    f.flush()
}
