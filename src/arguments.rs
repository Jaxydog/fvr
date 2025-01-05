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

//! Provides the command's arguments and implements a method for parsing them.

use std::{
    collections::HashSet,
    fmt::Display,
    io::Write,
    path::{Path, PathBuf},
};

use parse::Argument;

use self::parse::Parser;

pub mod parse;
pub mod schema;

/// Defines the command's outline.
pub const SCHEMA: self::schema::Command<'_> = {
    use self::schema::{Argument, Command, Value};

    Command {
        name: env!("CARGO_BIN_NAME"),
        about: env!("CARGO_PKG_DESCRIPTION"),
        version: Some(env!("CARGO_PKG_VERSION")),
        positionals: Some(&[Value {
            name: "PATHS",
            about: Some("The file paths to list"),
            list: true,
            required: false,
            default: Some("."),
            options: None,
        }]),
        arguments: Some(&[
            Argument { long: "help", short: Some('h'), about: "Shows the command's usage", value: None },
            Argument { long: "version", short: Some('V'), about: "Shows the command's version", value: None },
            Argument {
                long: "color",
                short: None,
                about: "Determines whether to use color",
                value: Some(Value {
                    name: "CHOICE",
                    about: None,
                    list: false,
                    required: true,
                    default: Some("auto"),
                    options: Some(&["auto", "always", "never"]),
                }),
            },
        ]),
        sub_commands: Some(&[Command {
            name: "tree",
            about: "List the contents of directories in a tree view",
            version: None,
            positionals: Some(&[Value {
                name: "PATHS",
                about: Some("The file paths to list"),
                list: true,
                required: false,
                default: Some("."),
                options: None,
            }]),
            arguments: Some(&[
                Argument { long: "help", short: Some('h'), about: "Shows the command's usage", value: None },
                Argument {
                    long: "color",
                    short: None,
                    about: "Determines whether to use color",
                    value: Some(Value {
                        name: "CHOICE",
                        about: None,
                        list: false,
                        required: true,
                        default: Some("auto"),
                        options: Some(&["auto", "always", "never"]),
                    }),
                },
            ]),
            sub_commands: None,
        }]),
    }
};

/// A result of trying to parse the application's command-line arguments.
#[derive(Clone, Debug)]
pub enum ParseResult {
    /// The arguments were successfully parsed.
    Ok(Arguments),
    /// Parsing failed and the program should exit with a code.
    Exit(u8),
}

/// The application's command-line arguments.
#[derive(Clone, Debug, Default)]
pub struct Arguments {
    /// The command to run.
    pub sub_command: SubCommand,
    /// Whether to print with color.
    pub color: ColorChoice,
}

impl Arguments {
    /// Prints a help listing to the given [`Write`][0] implementation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the listing could not be written.
    ///
    /// # Panics
    ///
    /// Panics if the [`SCHEMA`][1] constant is invalid.
    ///
    /// [0]: std::io::Write
    /// [1]: crate::arguments::SCHEMA
    pub fn write_help_listing(&self, f: &mut impl Write) -> std::io::Result<()> {
        #[expect(clippy::expect_used, reason = "we're directly grabbing values out of a known constant")]
        fn sub_command(index: usize) -> self::schema::Command<'static> {
            SCHEMA.sub_commands.and_then(|v| v.get(index).copied()).expect("missing required sub-command schema")
        }

        // We only validate for this debug builds to improve performance
        #[cfg(debug_assertions)]
        SCHEMA.validate();

        match self.sub_command {
            SubCommand::None | SubCommand::List { .. } => SCHEMA,
            SubCommand::Tree { .. } => sub_command(0),
        }
        .write_to(f)
    }
}

/// The application's sub-command.
#[non_exhaustive]
#[repr(i8)]
#[derive(Clone, Debug, Default)]
pub enum SubCommand {
    /// The command has not yet been set.
    #[default]
    None = -1,
    /// Display contents in a list view.
    List {
        /// The paths to list.
        paths: HashSet<Box<Path>>,
    } = 0,
    /// Display contents in a tree-based view.
    Tree {
        /// The paths to list.
        paths: HashSet<Box<Path>>,
    } = 1,
}

/// Determines whether to output using color.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorChoice {
    /// Automatically determine whether to use color.
    #[default]
    Auto,
    /// Always use color.
    Always,
    /// Never use color.
    Never,
}

/// Parses the application's command-line arguments from its invocation.
pub fn parse_args() -> ParseResult {
    let arguments: Box<[_]> = std::env::args().skip(1).collect();
    let mut parser = Parser::new(arguments.iter().map(String::as_str));

    let mut arguments = Arguments::default();

    while let Some(result) = parser.next_argument().transpose() {
        match result.map(|argument| self::handle_argument(&mut arguments, &mut parser, argument)) {
            Ok(None) => {}
            Ok(Some(result)) => return result,
            Err(error) => return self::exit_and_print(crate::exit_codes::ERROR_GENERIC, error),
        };
    }

    if matches!(&arguments.sub_command, SubCommand::None) {
        arguments.sub_command = SubCommand::List { paths: HashSet::new() };
    }

    match &mut arguments.sub_command {
        SubCommand::None => unreachable!(),
        SubCommand::List { paths } | SubCommand::Tree { paths } if paths.is_empty() => {
            match PathBuf::from(".").canonicalize() {
                Ok(path) => drop(paths.insert(path.into_boxed_path())),
                Err(error) => return self::exit_and_print(crate::exit_codes::ERROR_GENERIC, error),
            }
        }
        _ => {}
    }

    ParseResult::Ok(arguments)
}

/// Handles accepting arguments and applying their values to the arguments struct.
fn handle_argument<'p, I>(
    arguments: &mut Arguments,
    parser: &mut Parser<&'p str, I>,
    argument: Argument<&'p str>,
) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    use self::parse::Argument::{Long, Positional, Short};

    match argument {
        Short('h') | Long("help") => Some(self::exit_with_help(arguments, crate::exit_codes::SUCCESS)),

        Short('V') | Long("version") => Some(self::exit_and_print(
            crate::exit_codes::SUCCESS,
            format_args!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION")),
        )),

        Long("color") => {
            let Some(choice) = (match parser.next_value() {
                Ok(choice) => choice,
                Err(error) => return Some(self::exit_and_print(crate::exit_codes::ERROR_CLI_USAGE, error)),
            }) else {
                return Some(self::exit_and_print(crate::exit_codes::ERROR_CLI_USAGE, "expected color choice"));
            };

            arguments.color = match choice {
                "auto" => ColorChoice::Auto,
                "always" => ColorChoice::Always,
                "never" => ColorChoice::Never,
                _ => return Some(self::exit_and_print(crate::exit_codes::ERROR_CLI_USAGE, "invalid color choice")),
            };

            None
        }

        Positional(argument) => {
            if matches!(&arguments.sub_command, SubCommand::None) {
                match argument {
                    "tree" => {
                        arguments.sub_command = SubCommand::Tree { paths: HashSet::new() };

                        return None;
                    }
                    _ => arguments.sub_command = SubCommand::List { paths: HashSet::new() },
                }
            }

            match &mut arguments.sub_command {
                SubCommand::None => unreachable!(),
                SubCommand::List { paths } | SubCommand::Tree { paths } => match PathBuf::from(argument).canonicalize()
                {
                    Ok(path) => drop(paths.insert(path.into_boxed_path())),
                    Err(error) => return Some(self::exit_and_print(crate::exit_codes::ERROR_GENERIC, error)),
                },
            }

            None
        }

        argument => Some(self::exit_and_print(
            crate::exit_codes::ERROR_CLI_USAGE,
            format_args!("unexpected argument `{argument}`"),
        )),
    }
}

/// Return an exiting [`ParseResult`][0] and print the given value.
///
/// [0]: crate::arguments::ParseResult
#[inline]
fn exit_and_print(code: u8, display: impl Display) -> ParseResult {
    if code == crate::exit_codes::SUCCESS {
        println!("{display}");
    } else {
        eprintln!("{display}");
    }

    ParseResult::Exit(code)
}

/// Return an exiting [`ParseResult`][0] and print a help listing.
///
/// [0]: crate::arguments::ParseResult
#[inline]
fn exit_with_help(arguments: &Arguments, code: u8) -> ParseResult {
    let result = if code == crate::exit_codes::SUCCESS {
        arguments.write_help_listing(&mut std::io::stdout())
    } else {
        arguments.write_help_listing(&mut std::io::stderr())
    };

    match result {
        Ok(()) => ParseResult::Exit(code),
        Err(error) => self::exit_and_print(crate::exit_codes::ERROR_GENERIC, error),
    }
}
