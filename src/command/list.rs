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

use crate::arguments::model::{Arguments, SubCommand};
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

    let filter = crate::files::filter::by(|v, _| list_arguments.show_hidden || !is_hidden(v));
    let sort = list_arguments.sorting.clone().unwrap_or_default();
    let sort = sort.compile();

    let mode_section = (!list_arguments.mode.is_hide()).then(|| ModeSection::new(list_arguments.mode.is_extended()));
    let size_section = (!list_arguments.size.is_hide()).then(|| SizeSection::new(list_arguments.size));
    let created_section = (!list_arguments.created.is_hide()).then(|| CreatedSection::new(list_arguments.created));
    let modified_section = (!list_arguments.modified.is_hide()).then(|| ModifiedSection::new(list_arguments.modified));
    let user_section = list_arguments.user.then_some(UserSection);
    let group_section = list_arguments.group.then_some(GroupSection);
    let name_section = NameSection::new(true, list_arguments.resolve_symlinks);

    let f = &mut std::io::stdout().lock();

    for (index, path) in list_arguments.paths.get().enumerate() {
        let data = std::fs::symlink_metadata(path).ok();
        let entry = Rc::new(Entry::new(path, data.as_ref(), index, list_arguments.paths.len()));

        if list_arguments.paths.len() > 1 {
            if index > 0 {
                f.write_all(b"\n")?;
            }

            NameSection::new(true, false).write(arguments.color, f, &[], &entry)?;

            f.write_all(b":\n")?;
        }

        crate::files::visit_entries(&entry, &filter, &sort, |parents, entry| {
            if let Some(mode) = mode_section {
                mode.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(size) = size_section {
                size.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(created) = created_section {
                created.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(modified) = modified_section {
                modified.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(user) = user_section {
                user.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }
            if let Some(group) = group_section {
                group.write(arguments.color, f, parents, &entry)?;

                f.write_all(b" ")?;
            }

            name_section.write(arguments.color, f, parents, &entry)?;

            f.write_all(b"\n")
        })?;
    }

    f.flush()
}
