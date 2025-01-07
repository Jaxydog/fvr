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

use std::fmt::Display;
use std::path::Path;

use model::{ModeVisibility, SortingFunction};

use self::model::{Arguments, ColorChoice, ListArguments, SubCommand, TreeArguments};
use self::parse::{Argument, Parser};
use crate::exit_codes::{ERROR_CLI_USAGE, ERROR_GENERIC, SUCCESS};

pub mod model;
pub mod parse;
pub mod schema;

/// Defines the command's outline.
pub const SCHEMA: self::schema::Command<'static> = {
    use self::schema::{Argument, Command, Value};

    const HELP: Argument<'_> = Argument::new("help", "Shows the command's usage").short('h');
    const COLOR: Argument<'_> = Argument::new("color", "Determines whether to output with color")
        .value(Value::new("CHOICE").required().default("auto").options(&["auto", "always", "never"]));
    const ALL: Argument<'_> = Argument::new("all", "Show all entries, including hidden entries").short('a');
    const SORT: Argument<'_> = Argument::new("sort", "Determines the sorting order (comma separated)")
        .short('s')
        .value(Value::new("ORDER").required().list().default("name").options(&[
            "name",
            "created",
            "modified",
            "files",
            "symlinks",
            "directories",
            "hidden",
            "reverse-*",
        ]));
    const MODE: Argument<'_> = Argument::new("mode", "Determines whether to display an entry's Unix mode flags")
        .short('m')
        .value(Value::new("CHOICE").required().default("hide").options(&["hide", "show", "extended"]));

    Command::new(env!("CARGO_BIN_NAME"), env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arguments(&[HELP, Argument::new("version", "Shows the command's version").short('V'), COLOR])
        .sub_commands(&[
            Command::new("list", "List the contents of directories")
                .positionals(&[Value::new("PATHS").about("The file paths to list").list().default(".")])
                .arguments(&[HELP, COLOR, ALL, SORT, MODE]),
            Command::new("tree", "List the contents of directories in a tree-based view")
                .positionals(&[Value::new("PATHS").about("The file paths to list").list().default(".")])
                .arguments(&[HELP, COLOR, ALL, SORT]),
        ])
};

/// A result of trying to parse the application's command-line arguments.
pub enum ParseResult {
    /// The arguments were successfully parsed.
    Ok(Arguments),
    /// Parsing failed and the program should exit with a code.
    Exit(u8),
}

/// Return an exiting [`ParseResult`][0] and print the given value.
///
/// [0]: crate::arguments::ParseResult
#[inline]
fn exit_and_print(code: u8, display: impl Display) -> ParseResult {
    if code == SUCCESS {
        println!("{display}");
    } else {
        eprintln!("{display}");
    }

    ParseResult::Exit(code)
}

/// Parses the application's command-line arguments from its invocation.
pub fn parse_arguments() -> ParseResult {
    let arguments: Box<[_]> = std::env::args().skip(1).collect();
    let mut parser = Parser::new(arguments.iter().map(String::as_str));
    let mut arguments = Arguments::default();

    while let Some(result) = parser.next_argument().transpose() {
        if let Some(output) = match result {
            Ok(argument) => self::parse_argument(&mut arguments, &mut parser, argument),
            Err(error) => return self::exit_and_print(ERROR_GENERIC, error),
        } {
            return output;
        }
    }

    let Some(paths) = arguments.command.as_mut().map(|v| match v {
        SubCommand::List(arguments) => &mut arguments.paths,
        SubCommand::Tree(arguments) => &mut arguments.paths,
    }) else {
        return self::exit_and_print(ERROR_CLI_USAGE, "no sub-command was provided");
    };

    if paths.is_empty() {
        match std::env::current_dir().and_then(|v| v.canonicalize()) {
            Ok(path) => paths.add(path),
            Err(error) => return self::exit_and_print(ERROR_GENERIC, error),
        }
    }

    ParseResult::Ok(arguments)
}

/// Parses a single command-line argument.
fn parse_argument<'p, I>(
    arguments: &mut Arguments,
    parser: &mut Parser<&'p str, I>,
    argument: Argument<&'p str>,
) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    use self::parse::Argument::{Long, Positional, Short};

    match argument {
        Short('h') | Long("help") => Some(self::parse_help(arguments, parser)),
        Short('V') | Long("version") if arguments.command.is_none() => Some(self::parse_version()),
        Long("color") => self::parse_color(arguments, parser),
        Short('a') | Long("all") if arguments.command.is_some() => self::parse_all(arguments),
        Short('s') | Long("sort") if arguments.command.is_some() => self::parse_sort(arguments, parser),
        Short('m') | Long("mode") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_mode(arguments, parser)
        }
        Positional(value) => self::parse_positional(arguments, value),
        _ => Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("unexpected argument `{argument}`"))),
    }
}

/// Parses a single positional command-line argument.
fn parse_positional(arguments: &mut Arguments, value: &str) -> Option<ParseResult> {
    if let Some(command) = arguments.command.as_mut() {
        let (SubCommand::List(ListArguments { paths, .. }) | SubCommand::Tree(TreeArguments { paths, .. })) = command;

        match Path::new(value).canonicalize() {
            Ok(path) => paths.add(path),
            Err(error) => return Some(self::exit_and_print(ERROR_GENERIC, error)),
        }
    } else {
        arguments.command = Some(match value {
            "list" => SubCommand::List(ListArguments::default()),
            "tree" => SubCommand::Tree(TreeArguments::default()),
            _ => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("unknown sub-command `{value}`"))),
        });
    }

    None
}

/// Parses the help command-line argument.
fn parse_help<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> ParseResult
where
    I: Iterator<Item = &'p str>,
{
    if let Ok(Some(value)) = arguments.command.is_none().then(|| parser.next_value()).transpose().map(Option::flatten) {
        // Attempt to read the next argument as a sub-command.
        drop(self::parse_positional(arguments, value));
    }

    match arguments.current_schema().write_to(&mut std::io::stdout()) {
        Ok(()) => ParseResult::Exit(SUCCESS),
        Err(error) => self::exit_and_print(ERROR_GENERIC, error),
    }
}

/// Parses the version command-line argument.
fn parse_version() -> ParseResult {
    self::exit_and_print(SUCCESS, format_args!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION")))
}

/// Parses the color command-line argument.
fn parse_color<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "expected color choice"));
    };

    arguments.color = match choice {
        "auto" => ColorChoice::Auto,
        "always" => ColorChoice::Always,
        "never" => ColorChoice::Never,
        _ => return Some(self::exit_and_print(ERROR_CLI_USAGE, "invalid color choice")),
    };

    None
}

/// Parses the all command-line argument.
fn parse_all(arguments: &mut Arguments) -> Option<ParseResult> {
    let Some(command) = arguments.command.as_mut() else { unreachable!() };

    match command {
        SubCommand::List(arguments) => arguments.show_hidden = true,
        SubCommand::Tree(arguments) => arguments.show_hidden = true,
    }

    None
}

/// Parses the sort command-line argument.
fn parse_sort<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(SubCommand::List(ListArguments { sorting, .. }) | SubCommand::Tree(TreeArguments { sorting, .. })) =
        arguments.command.as_mut()
    else {
        unreachable!();
    };

    let Some(orderings) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "expected sort ordering string"));
    };

    *sorting = None;

    for ordering in orderings.split(',') {
        use crate::files::sorting::{created, directories, files, hidden, modified, name, reverse, symlinks, then};

        let mut function = match ordering.trim_start_matches("reverse-") {
            "name" => SortingFunction::new(name()),
            "created" => SortingFunction::new(created()),
            "modified" => SortingFunction::new(modified()),
            "files" => SortingFunction::new(files()),
            "symlinks" => SortingFunction::new(symlinks()),
            "directories" => SortingFunction::new(directories()),
            "hidden" => SortingFunction::new(hidden()),
            _ => return Some(self::exit_and_print(ERROR_CLI_USAGE, "invalid ordering string")),
        };

        if ordering.starts_with("reverse-") {
            function = SortingFunction::new(reverse(function.unpack()));
        }

        if let Some(current) = sorting.take().map(|v| v.unpack()) {
            *sorting = Some(SortingFunction::new(then(current, function.unpack())));
        } else {
            *sorting = Some(function);
        }
    }

    None
}

/// Parses the mode command-line argument.
fn parse_mode<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "expected mode visibility"));
    };

    let Some(SubCommand::List(ListArguments { mode, .. })) = arguments.command.as_mut() else { unreachable!() };

    *mode = match choice {
        "hide" => ModeVisibility::Hide,
        "show" => ModeVisibility::Show,
        "extended" => ModeVisibility::Extended,
        _ => return Some(self::exit_and_print(ERROR_CLI_USAGE, "invalid mode visibility")),
    };

    None
}
