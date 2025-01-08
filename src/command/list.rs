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

use std::fs::Metadata;
use std::io::Write;

use crate::arguments::model::{Arguments, ModeVisibility, SubCommand};
use crate::display::mode::Mode;
use crate::display::name::Name;
use crate::display::size::Size;
use crate::display::time::Time;
use crate::display::user::{Group, User};
use crate::display::{Show, ShowData};
use crate::files::is_hidden;

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(arguments: &Arguments) -> std::io::Result<()> {
    let Some(SubCommand::List(list_arguments)) = arguments.command.as_ref() else { unreachable!() };

    let filter = crate::files::filter::by(|v, _| list_arguments.show_hidden || !is_hidden(v));
    let sorter = list_arguments.sorting.clone().unwrap_or_default();
    let sorter = sorter.compile();

    let show_name = Name::new(list_arguments.resolve_symlinks, true);
    let show_mode = match list_arguments.mode {
        ModeVisibility::Hide => None,
        ModeVisibility::Show => Some(Mode::new(false)),
        ModeVisibility::Extended => Some(Mode::new(true)),
    };
    let show_size = (!list_arguments.size.is_hide()).then(|| Size::new(list_arguments.size));
    let show_created =
        (!list_arguments.created.is_hide()).then(|| Time::new(list_arguments.created, Metadata::created));
    let show_modified =
        (!list_arguments.modified.is_hide()).then(|| Time::new(list_arguments.modified, Metadata::modified));
    let show_user = list_arguments.user.then_some(User);
    let show_group = list_arguments.group.then_some(Group);

    let f = &mut std::io::stdout().lock();

    for (index, path) in list_arguments.paths.get().enumerate() {
        let remaining = list_arguments.paths.len() - index;
        let root_entry = ShowData { path, data: None, remaining, depth: None };

        if list_arguments.paths.len() > 1 {
            if index > 0 {
                f.write_all(b"\n")?;
            }

            show_name.show(arguments, f, root_entry)?;

            f.write_all(b":\n")?;
        }

        crate::files::visit(path, &filter, &sorter, |path, data, remaining| {
            let entry = ShowData { path, data: Some(data), remaining, depth: None };

            if let Some(mode) = show_mode {
                mode.show(arguments, f, entry)?;

                f.write_all(b" ")?;
            }

            if let Some(size) = show_size {
                size.show(arguments, f, entry)?;

                f.write_all(b" ")?;
            }

            if let Some(created) = show_created {
                created.show(arguments, f, entry)?;

                f.write_all(b" ")?;
            }

            if let Some(modified) = show_modified {
                modified.show(arguments, f, entry)?;

                f.write_all(b" ")?;
            }

            if let Some(user) = show_user {
                user.show(arguments, f, entry)?;

                f.write_all(b" ")?;
            }

            if let Some(group) = show_group {
                group.show(arguments, f, entry)?;

                f.write_all(b" ")?;
            }

            show_name.show(arguments, f, entry)?;

            f.write_all(b"\n")
        })?;
    }

    Ok(())
}
