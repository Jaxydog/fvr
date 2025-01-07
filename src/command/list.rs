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
use std::os::unix::fs::MetadataExt;

use super::super::arguments::model::SortingFunction;
use crate::arguments::model::{Arguments, ListArguments, ModeVisibility};
use crate::display::Rendered;
use crate::display::mode::Mode;
use crate::display::name::Name;

/// Runs the command.
///
/// # Errors
///
/// This function will return an error if the command fails.
pub fn invoke(arguments: &Arguments, list_arguments: &ListArguments) -> std::io::Result<()> {
    let filter = crate::files::filtering::visible(list_arguments.show_hidden);
    let sorter = list_arguments.sorting.as_ref().map(SortingFunction::get);

    let should_show_permissions = match list_arguments.mode {
        ModeVisibility::Hide => None,
        ModeVisibility::Show => Some(false),
        ModeVisibility::Extended => Some(true),
    };

    let f = &mut std::io::stdout().lock();

    for (index, path) in list_arguments.paths.get().enumerate() {
        if list_arguments.paths.len() > 1 {
            if index > 0 {
                f.write_all(b"\n")?;
            }

            Name::new(path, false).show(arguments, f)?;

            f.write_all(b":\n")?;
        }

        crate::files::visit_directory(path, sorter, filter, |path, _remaining| {
            if let Some(extended) = should_show_permissions {
                let mode = path.symlink_metadata()?.mode();

                Mode::new(mode, extended).show(arguments, f)?;

                f.write_all(b" ")?;
            }

            Name::new(path, true).show(arguments, f)?;

            f.write_all(b"\n")
        })?;
    }

    Ok(())
}
