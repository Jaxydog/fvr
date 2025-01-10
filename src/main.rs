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

//! An implementation of the `ls` command-line application.

// Panic prevention
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![cfg_attr(debug_assertions, warn(clippy::todo, clippy::unimplemented))]
#![cfg_attr(not(debug_assertions), deny(clippy::todo, clippy::unimplemented))]
// Safety checks
#![deny(unsafe_code, clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)]
// General lints
#![warn(clippy::cargo, clippy::nursery, clippy::pedantic, missing_docs)]
// Feature gates
#![feature(can_vector, slice_split_once, try_trait_v2, write_all_vectored)]

use std::process::ExitCode;

use arguments::model::SubCommand;

use self::arguments::ParseResult;

pub mod arguments;
pub mod files;
pub mod section;

/// Defines sub-command implementations.
pub mod command {
    pub mod list;
    pub mod tree;
}

/// Defines the application's constant exit codes.
pub mod exit_codes {
    /// The program ran successfully.
    pub const SUCCESS: u8 = 0;
    /// A generic error was encountered.
    pub const ERROR_GENERIC: u8 = 1;
    /// An invalid argument or number of arguments were provided.
    pub const ERROR_CLI_USAGE: u8 = 2;
}

fn main() -> ExitCode {
    let arguments = match self::arguments::parse_arguments() {
        ParseResult::Ok(arguments) => arguments,
        ParseResult::Exit(code) => return ExitCode::from(code),
    };

    if let Err(error) = match &arguments.command {
        None => unreachable!(),
        Some(SubCommand::List(_)) => self::command::list::invoke(&arguments),
        Some(SubCommand::Tree(_)) => self::command::tree::invoke(&arguments),
    } {
        eprintln!("{error}");

        return ExitCode::from(self::exit_codes::ERROR_GENERIC);
    };

    ExitCode::from(self::exit_codes::SUCCESS)
}
