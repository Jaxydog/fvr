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

    let displays: &[&dyn Show] = &[
        &match list_arguments.mode {
            ModeVisibility::Hide => None,
            ModeVisibility::Show => Some(Mode::new(false)),
            ModeVisibility::Extended => Some(Mode::new(true)),
        },
        &(!list_arguments.size.is_hide()).then(|| Size::new(list_arguments.size)),
        &(!list_arguments.created.is_hide()).then(|| Time::new(list_arguments.created, Metadata::created)),
        &(!list_arguments.modified.is_hide()).then(|| Time::new(list_arguments.modified, Metadata::modified)),
        &list_arguments.user.then_some(User),
        &list_arguments.group.then_some(Group),
        &Name::new(list_arguments.resolve_symlinks, true),
    ];

    let f = &mut std::io::stdout().lock();

    for (index, path) in list_arguments.paths.get().enumerate() {
        let count = list_arguments.paths.len();
        let root_entry = ShowData { path, data: None, index, count, depth: None };

        if list_arguments.paths.len() > 1 {
            if index > 0 {
                f.write_all(b"\n")?;
            }

            Name::new(false, false).show(arguments, f, root_entry)?;

            f.write_all(b":\n")?;
        }

        crate::files::visit(path, &filter, &sorter, |path, data, index, count| {
            let entry = ShowData { path, data: Some(data), index, count, depth: None };

            displays.show(arguments, f, entry).and_then(|()| f.write_all(b"\n"))
        })?;
    }

    f.flush()
}
