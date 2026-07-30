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

//! Implements the list subcommand.

use std::fs::Metadata;
use std::io::Write;
use std::path::Path;

use recomposition::sort::ListSortExt;

use crate::arguments::model::{Arguments, SubCommand};
use crate::files::{Entry, is_hidden};
use crate::section::Section;
use crate::section::name::NameSection;

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(mut arguments: Arguments) -> std::io::Result<()> {
    let Some(SubCommand::List(list_arguments)) = arguments.command else { unreachable!() };

    let name_section = NameSection::new(true, arguments.resolve_symlinks);

    let sort = arguments.sort_order.get_or_insert_default();
    let filter = recomposition::filter::from_fn(|(path, _): &(Box<Path>, _)| {
        (arguments.show_hidden || !is_hidden(path))
            && arguments.included.as_ref().is_none_or(|include| include.contains(path))
            && !arguments.excluded.as_ref().is_some_and(|exclude| exclude.contains(path))
    });

    let total_paths = arguments.paths.len();
    let paths = arguments.paths.into_iter().map(|path| {
        let data = std::fs::symlink_metadata(&path)?;

        Ok((path, data))
    });

    let mut paths = paths.collect::<std::io::Result<Box<[(Box<Path>, Metadata)]>>>()?;

    paths.sort_unstable_with(&sort);

    let f = &mut std::io::stdout().lock();

    for (index, (path, data)) in paths.into_iter().enumerate() {
        let entry = Entry::new(path, Some(&data), index, total_paths, &filter);

        if index > 0 {
            f.write_all(b"\n")?;
        }
        if total_paths > 1 {
            if entry.can_traverse() {
                name_section.write(arguments.color, f, &[], &entry)?;
            } else {
                let path = entry.path.absolute()?.parent().map_or_else(|| Path::new("/").into(), Box::from);

                name_section.write(arguments.color, f, &[], &Entry::root(path, None, &filter))?;
            }

            f.write_all(b":\n")?;
        }

        crate::files::visit_entries(&entry, &filter, &sort, |parents, entry| {
            if let Some(mode) = list_arguments.mode {
                mode.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }
            if let Some(size) = list_arguments.size {
                size.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }
            if let Some(created) = list_arguments.created {
                created.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }
            if let Some(accessed) = list_arguments.accessed {
                accessed.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }
            if let Some(modified) = list_arguments.modified {
                modified.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }
            if let Some(user) = &list_arguments.user {
                user.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }
            if let Some(group) = &list_arguments.group {
                group.write(arguments.color, f, parents, entry)?;

                f.write_all(b" ")?;
            }

            name_section.write(arguments.color, f, parents, entry)?;

            f.write_all(b"\n")
        })?;
    }

    f.flush()
}
