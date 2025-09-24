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
use std::num::IntErrorKind;
use std::path::Path;

use self::model::{
    Arguments, ColorChoice, ListArguments, ModeVisibility, SizeVisibility, SortOrder, SubCommand, TimeVisibility,
    TreeArguments,
};
use self::parse::{Argument, Parser};
use crate::arguments::schema::{
    ArgumentSchema, ArgumentSchemaBuilder, CommandSchema, CommandSchemaBuilder, ValueSchema, ValueSchemaBuilder,
};
use crate::exit_codes::{ERROR_CLI_USAGE, ERROR_GENERIC, SUCCESS};
use crate::section::time::TimeSectionType;

pub mod model;
pub mod parse;
pub mod schema;

/// Defines the command's schema.
pub const SCHEMA: CommandSchema<'static> = {
    const PATHS_VALUE: ValueSchema<'static> =
        ValueSchemaBuilder::new("PATHS").about("The paths to display").list().build();
    const PATH_VALUE: ValueSchema<'static> = ValueSchemaBuilder::new("PATH").about("The path").required().build();
    const COLOR_VALUE: ValueSchema<'static> =
        ValueSchemaBuilder::new("CHOICE").required().default("auto").options(&["auto", "always", "never"]).build();
    const SORT_ORDER_VALUE: ValueSchema<'static> = ValueSchemaBuilder::new("ORDER")
        .required()
        .list()
        .default("directories,files,name")
        .options(&[
            "name",
            "accessed",
            "created",
            "modified",
            "size",
            "files",
            "symlinks",
            "directories",
            "hidden",
            "reverse-*",
        ])
        .build();

    const HELP_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("help", "Shows the command's usage").short('h').build();
    const COLOR_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("color", "Determines whether to output using color").value(COLOR_VALUE).build();
    const ALL_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("all", "Include hidden files and directories").short('a').build();
    const EXCLUDE_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("exclude", "Exclude a directory from output").short('e').value(PATH_VALUE).build();
    const INCLUDE_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("include", "Include a directory in the output").short('i').value(PATH_VALUE).build();
    const RESOLVE_SYMLINKS_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("resolve-symlinks", "Fully resolve symbolic link paths").short('r').build();
    const SORT_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("sort", "Control how entries are sorted").value(SORT_ORDER_VALUE).build();

    const MODE_VALUE: ValueSchema<'static> =
        ValueSchemaBuilder::new("CHOICE").required().default("hide").options(&["hide", "show", "extended"]).build();
    const SIZE_VALUE: ValueSchema<'static> = ValueSchemaBuilder::new("CHOICE")
        .required()
        .default("hide")
        .options(&["hide", "simple", "base-2", "base-10"])
        .build();
    const TIME_VALUE: ValueSchema<'static> =
        ValueSchemaBuilder::new("CHOICE").required().default("hide").options(&["hide", "simple", "iso8601"]).build();
    const DEPTH_VALUE: ValueSchema<'static> = ValueSchemaBuilder::new("DEPTH").required().build();

    const MODE_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("mode", "Control how entry modes are shown").short('m').value(MODE_VALUE).build();
    const SIZE_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("size", "Control how entry sizes are shown").short('s').value(SIZE_VALUE).build();
    const CREATED_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("created", "Control how creation dates are shown").value(TIME_VALUE).build();
    const ACCESSED_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("accessed", "Control how access dates are shown").value(TIME_VALUE).build();
    const MODIFIED_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("modified", "Control how modification dates are shown").value(TIME_VALUE).build();
    const USER_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("user", "Show all entry user names").build();
    const GROUP_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("group", "Show all entry group names").build();
    const DEPTH_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("depth", "Control how deep to traverse").short('d').value(DEPTH_VALUE).build();

    const LIST_COMMAND: CommandSchema<'static> =
        CommandSchemaBuilder::new("list", "List the contents of one or more directories")
            .positionals(&[PATHS_VALUE])
            .arguments(&[
                HELP_ARGUMENT,
                COLOR_ARGUMENT,
                ALL_ARGUMENT,
                EXCLUDE_ARGUMENT,
                INCLUDE_ARGUMENT,
                RESOLVE_SYMLINKS_ARGUMENT,
                SORT_ARGUMENT,
                MODE_ARGUMENT,
                SIZE_ARGUMENT,
                CREATED_ARGUMENT,
                ACCESSED_ARGUMENT,
                MODIFIED_ARGUMENT,
                USER_ARGUMENT,
                GROUP_ARGUMENT,
            ])
            .build();

    const TREE_COMMAND: CommandSchema<'static> =
        CommandSchemaBuilder::new("tree", "List the contents of one or more directories in a tree-based view")
            .positionals(&[PATHS_VALUE])
            .arguments(&[
                HELP_ARGUMENT,
                COLOR_ARGUMENT,
                ALL_ARGUMENT,
                INCLUDE_ARGUMENT,
                EXCLUDE_ARGUMENT,
                RESOLVE_SYMLINKS_ARGUMENT,
                SORT_ARGUMENT,
                DEPTH_ARGUMENT,
            ])
            .build();

    const SUBCOMMAND_VALUE: ValueSchema<'static> =
        ValueSchemaBuilder::new("SUBCOMMAND").options(&[LIST_COMMAND.name, TREE_COMMAND.name]).build();

    const HELP_WITH_SUBCOMMAND_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("help", "Shows the command (or a sub-command)'s usage")
            .short('h')
            .value(SUBCOMMAND_VALUE)
            .build();
    const VERSION_ARGUMENT: ArgumentSchema<'static> =
        ArgumentSchemaBuilder::new("version", "Shows the command's version").short('V').build();

    CommandSchemaBuilder::new(env!("CARGO_BIN_NAME"), env!("CARGO_PKG_DESCRIPTION"))
        .version(env!("CARGO_PKG_VERSION"))
        .arguments(&[HELP_WITH_SUBCOMMAND_ARGUMENT, VERSION_ARGUMENT, COLOR_ARGUMENT])
        .commands(&[LIST_COMMAND, TREE_COMMAND])
}
.build();

/// A result of trying to parse the application's command-line arguments.
pub enum ParseResult {
    /// The arguments were successfully parsed.
    Ok(Arguments),
    /// Parsing failed and the program should exit with a code.
    Exit(u8),
}

/// Return an exiting [`ParseResult`] and print the given value.
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
            Ok(path) => paths.add(path.into_boxed_path()),
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
        Short('r') | Long("resolve-symlinks") if arguments.command.is_some() => self::parse_resolve_symlinks(arguments),
        Long("sort") if arguments.command.is_some() => self::parse_sort(arguments, parser),
        Short('m') | Long("mode") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_mode(arguments, parser)
        }
        Short('s') | Long("size") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_size(arguments, parser)
        }
        Long("created") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_time(arguments, parser, TimeSectionType::Created)
        }
        Long("accessed") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_time(arguments, parser, TimeSectionType::Accessed)
        }
        Long("modified") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_time(arguments, parser, TimeSectionType::Modified)
        }
        Short('u') | Long("user") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_user(arguments)
        }
        Short('g') | Long("group") if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_group(arguments)
        }
        Short('e') | Long("exclude") if arguments.command.is_some() => self::parse_exclude(arguments, parser),
        Short('i') | Long("include") if arguments.command.is_some() => self::parse_include(arguments, parser),
        Short('d') | Long("depth") if arguments.command.as_ref().is_some_and(SubCommand::is_tree) => {
            self::parse_depth(arguments, parser)
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
            Ok(path) => paths.add(path.into_boxed_path()),
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

    match self::schema::write_help(arguments.current_schema(), &mut std::io::stdout()) {
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
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing color choice"));
    };

    arguments.color = match choice {
        "auto" => ColorChoice::Auto,
        "always" => ColorChoice::Always,
        "never" => ColorChoice::Never,
        v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid color choice '{v}'"))),
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

/// Parses the resolve-symlinks command-line argument.
fn parse_resolve_symlinks(arguments: &mut Arguments) -> Option<ParseResult> {
    let Some(command) = arguments.command.as_mut() else { unreachable!() };

    match command {
        SubCommand::List(arguments) => arguments.resolve_symlinks = true,
        SubCommand::Tree(arguments) => arguments.resolve_symlinks = true,
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
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing sort order"));
    };

    *sorting = None;

    for string in orderings.split(',') {
        let mut next = match string.trim_start_matches("reverse-") {
            "name" => SortOrder::Name,
            "accessed" => SortOrder::Accessed,
            "created" => SortOrder::Created,
            "modified" => SortOrder::Modified,
            "size" => SortOrder::Size,
            "files" => SortOrder::Files,
            "symlinks" => SortOrder::Symlinks,
            "directories" => SortOrder::Directories,
            "hidden" => SortOrder::Hidden,
            v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid sort order '{v}'"))),
        };

        if string.starts_with("reverse-") {
            next = next.reverse();
        }

        if let Some(current) = sorting.take().filter(|v| v.top() != &next) {
            *sorting = Some(current.then(next));
        } else {
            *sorting = Some(next);
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
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing mode visibility"));
    };

    let Some(SubCommand::List(ListArguments { mode, .. })) = arguments.command.as_mut() else { unreachable!() };

    *mode = match choice {
        "hide" => ModeVisibility::Hide,
        "show" => ModeVisibility::Show,
        "extended" => ModeVisibility::Extended,
        v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid mode visibility '{v}'"))),
    };

    None
}

/// Parses the size command-line argument.
fn parse_size<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing size visibility"));
    };

    let Some(SubCommand::List(ListArguments { size, .. })) = arguments.command.as_mut() else { unreachable!() };

    *size = match choice {
        "hide" => SizeVisibility::Hide,
        "simple" => SizeVisibility::Simple,
        "base-2" => SizeVisibility::Base2,
        "base-10" => SizeVisibility::Base10,
        v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid size visibility '{v}'"))),
    };

    None
}

/// Parses the created, accessed, and/or modified command-line argument.
fn parse_time<'p, I>(
    arguments: &mut Arguments,
    parser: &mut Parser<&'p str, I>,
    kind: TimeSectionType,
) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing time visibility"));
    };

    let choice = match choice {
        "hide" => TimeVisibility::Hide,
        "simple" => TimeVisibility::Simple,
        "iso8601" => TimeVisibility::Iso8601,
        v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid time visibility '{v}'"))),
    };

    let Some(SubCommand::List(ListArguments { created, accessed, modified, .. })) = arguments.command.as_mut() else {
        unreachable!();
    };

    match kind {
        TimeSectionType::Created => *created = choice,
        TimeSectionType::Accessed => *accessed = choice,
        TimeSectionType::Modified => *modified = choice,
    }

    None
}

/// Parses the user command-line argument.
fn parse_user(arguments: &mut Arguments) -> Option<ParseResult> {
    let Some(command) = arguments.command.as_mut() else { unreachable!() };

    match command {
        SubCommand::List(arguments) => arguments.user = true,
        SubCommand::Tree(_) => unreachable!(),
    }

    None
}

/// Parses the group command-line argument.
fn parse_group(arguments: &mut Arguments) -> Option<ParseResult> {
    let Some(command) = arguments.command.as_mut() else { unreachable!() };

    match command {
        SubCommand::List(arguments) => arguments.group = true,
        SubCommand::Tree(_) => unreachable!(),
    }

    None
}

/// Parses the exclude command-line argument.
fn parse_exclude<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(path) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing excluded path"));
    };
    let path = match std::fs::canonicalize(path) {
        Ok(path) => path.into_boxed_path(),
        Err(error) => return Some(self::exit_and_print(ERROR_GENERIC, error)),
    };

    match arguments.command.as_mut() {
        None => unreachable!(),
        Some(SubCommand::List(arguments)) => arguments.excluded.get_or_insert_default().add(path),
        Some(SubCommand::Tree(arguments)) => arguments.excluded.get_or_insert_default().add(path),
    }

    None
}

/// Parses the include command-line argument.
fn parse_include<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(path) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing included path"));
    };
    let path = match std::fs::canonicalize(path) {
        Ok(path) => path.into_boxed_path(),
        Err(error) => return Some(self::exit_and_print(ERROR_GENERIC, error)),
    };

    match arguments.command.as_mut() {
        None => unreachable!(),
        Some(SubCommand::List(arguments)) => arguments.included.get_or_insert_default().add(path),
        Some(SubCommand::Tree(arguments)) => arguments.included.get_or_insert_default().add(path),
    }

    None
}

/// Parses the depth command-line argument.
fn parse_depth<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.next_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing traversal depth"));
    };

    let Some(SubCommand::Tree(TreeArguments { max_depth, .. })) = arguments.command.as_mut() else { unreachable!() };

    *max_depth = Some(match choice.parse() {
        Ok(value) => value,
        Err(error) => {
            return Some(self::exit_and_print(ERROR_CLI_USAGE, match error.kind() {
                IntErrorKind::Empty => "missing traversal depth",
                IntErrorKind::Zero | IntErrorKind::InvalidDigit => "depth must be a non-zero positive integer",
                IntErrorKind::PosOverflow => "depth is too large",
                IntErrorKind::NegOverflow => "depth is too small",
                _ => "invalid depth",
            }));
        }
    });

    None
}
