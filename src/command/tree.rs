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

use crate::arguments::model::{Arguments, SubCommand};
use crate::display::name::Name;
use crate::display::{Show, ShowData};
use crate::files::is_hidden;

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(arguments: &Arguments) -> std::io::Result<()> {
    let Some(SubCommand::Tree(tree_arguments)) = arguments.command.as_ref() else { unreachable!() };

    let filter = crate::files::filter::by(|v, _| tree_arguments.show_hidden || !is_hidden(v));
    let sorter = tree_arguments.sorting.clone().unwrap_or_default();
    let sorter = sorter.compile();

    let name = Name::new(tree_arguments.resolve_symlinks, true);

    let f = &mut std::io::stdout().lock();

    for (index, path) in tree_arguments.paths.get().enumerate() {
        let count = tree_arguments.paths.len();
        let root_entry = ShowData { path, data: None, index, count, depth: None };

        if index > 0 {
            f.write_all(b"\n")?;
        }

        Name::new(false, true).show(arguments, f, root_entry)?;

        f.write_all(b":\n")?;

        crate::files::visit_recursive(path, &filter, &sorter, 0, &mut |path, data, index, count, depth| {
            let entry = ShowData { path, data: Some(data), index, count, depth: Some(depth) };

            name.show(arguments, f, entry).and_then(|()| f.write_all(b"\n"))
        })?;
    }

    f.flush()
}
