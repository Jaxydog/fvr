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

//! Implements the list sub-command.

use std::io::Write;
use std::rc::Rc;

use crate::arguments::model::{Arguments, ListArguments, SubCommand};
use crate::files::{Entry, is_hidden};
use crate::section::Section;
use crate::section::mode::ModeSection;
use crate::section::name::NameSection;
use crate::section::size::SizeSection;
use crate::section::time::{CreatedSection, ModifiedSection};
use crate::section::user::{GroupSection, UserSection};

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(arguments: &Arguments) -> std::io::Result<()> {
    let Some(SubCommand::List(list_arguments)) = arguments.command.as_ref() else { unreachable!() };
    let ListArguments { paths, show_hidden, resolve_symlinks, sorting, mode, size, created, modified, user, group } =
        list_arguments;

    let filter = crate::files::filter::by(|v, _| *show_hidden || !is_hidden(v));
    let sort = sorting.clone().unwrap_or_default();
    let sort = sort.compile();

    let mode_section = if mode.is_hide() { None } else { Some(ModeSection::new(mode.is_extended())) };
    let size_section = if size.is_hide() { None } else { Some(SizeSection::new(*size)) };
    let created_section = if created.is_hide() { None } else { Some(CreatedSection::new(*created)) };
    let modified_section = if modified.is_hide() { None } else { Some(ModifiedSection::new(*modified)) };
    let user_section = user.then_some(UserSection);
    let group_section = group.then_some(GroupSection);
    let name_section = NameSection::new(true, *resolve_symlinks);

    let f = &mut std::io::stdout().lock();

    for (index, path) in paths.get().enumerate() {
        let data = std::fs::symlink_metadata(path).ok();
        let entry = Rc::new(Entry::new(path, data.as_ref(), index, paths.len()));

        if paths.len() > 1 {
            if index > 0 {
                f.write_all(b"\n")?;
            }

            NameSection::new(true, false).write(arguments.color, f, &[], &entry)?;

            f.write_all(b":\n")?;
        }

        crate::files::visit_entries(&entry, &filter, &sort, |parents, entry| {
            if let Some(mode) = &mode_section {
                mode.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(size) = &size_section {
                size.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(created) = &created_section {
                created.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(modified) = &modified_section {
                modified.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(user) = &user_section {
                user.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(group) = &group_section {
                group.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }

            name_section.write(arguments.color, f, parents, &entry)?;

            f.write_all(b"\n")
        })?;
    }

    f.flush()
}
