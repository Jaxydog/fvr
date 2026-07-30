// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025–2026 Jaxydog
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
use std::path::{Path, PathBuf};

use carp::{ArgumentOrPositional, Parser};

use self::model::{
    Arguments, ColorChoice, ListArguments, SizeVisibility, SortOrder, SubCommand, TimeVisibility, TreeArguments,
};
use crate::exit_codes::{ERROR_CLI_USAGE, ERROR_GENERIC, SUCCESS};
use crate::section::mode::ModeSection;
use crate::section::size::SizeSection;
use crate::section::time::{TimeSection, TimeSectionType};
use crate::section::user::{GroupSection, UserSection};

pub mod model;

/// The text displayed when `--help` is used.
pub const HELP: &str = concat!(
    env!("CARGO_PKG_DESCRIPTION"),
    "\n\nUsage: ",
    env!("CARGO_BIN_NAME"),
    " <subcommand> <arguments>

Subcommands:
  list                          List the contents of one or more directories
  tree                          List the contents of one or more directories in a tree-like view

Arguments:
  -h, --help                    Displays the command's usage
  -V, --version                 Displays the command's version

      --color <choice>          Determines whether the command should output ANSI color codes
                                - default: auto
                                - options: auto, always, never

  -a, --all                     Determines whether hidden files and directories should be displayed

  -e, --exclude <path>          Exclude a path from the command output
  -i, --include <path>          Include a path in the command output

  -r, --resolve-symlinks        Fully resolve symbolic link paths

      --sort <order>            Determines the sorting order of each displayed entry, accepts a comma-separated list
                                - default: directories,files,name
                                - options: name, accessed, created, modified, size, files, symlinks, directories, \
     hidden, reverse-*

List Arguments:
  -m, --mode <visibility>       Determines if and how the file mode of each entry is displayed
                                - default: hide
                                - options: hide, show, extended

  -s, --size <visibility>       Determines if and how the size of each entry is displayed
                                - default: hide
                                - options: hide, simple, base-2, base-10

      --created <visibility>    Determines if and how the creation date of each entry is displayed
                                - default: hide
                                - options: hide, simple, iso8601
      --accessed <visibility>   Determines if and how the last access date of each entry is displayed
                                - default: hide
                                - options: hide, simple, iso8601
      --modified <visibility>   Determines if and how the last modification date of each entry is displayed
                                - default: hide
                                - options: hide, simple, iso8601

      --user                    Display the name of each entry's owner
      --group                   Display the name of each entry's owner group

Tree Arguments:
  -d, --depth <depth>           Determines how many layers deep the tree should display"
);

/// A result of trying to parse the application's command-line arguments.
pub enum ParseResult {
    /// The arguments were successfully parsed.
    Ok(Arguments),
    /// Parsing failed and the program should exit with a code.
    Exit(u8),
}

/// Return a [`ParseResult::Exit`] and print the given value.
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

    while let Some(argument_or_positional) = match parser.parse_next() {
        Ok(argument_or_positional) => argument_or_positional,
        Err(error) => return self::exit_and_print(ERROR_CLI_USAGE, error),
    } {
        if let Some(output) = self::parse_argument(&mut arguments, &mut parser, argument_or_positional) {
            return output;
        }
    }

    if arguments.command.is_none() {
        return self::exit_and_print(ERROR_CLI_USAGE, "no subcommand was provided");
    }

    if arguments.paths.is_empty() {
        match std::env::current_dir().and_then(|v| v.canonicalize()) {
            Ok(path) => arguments.paths.push(path.into_boxed_path()),
            Err(error) => return self::exit_and_print(ERROR_GENERIC, error),
        }
    }

    ParseResult::Ok(arguments)
}

/// Parses a single command-line argument.
fn parse_argument<'p, I>(
    arguments: &mut Arguments,
    parser: &mut Parser<&'p str, I>,
    argument: ArgumentOrPositional<&'p str>,
) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    use carp::Argument::{Long, Short};
    use carp::ArgumentOrPositional::{Argument, Positional};

    match argument {
        Argument(Short('h') | Long("help")) => Some(self::parse_help(arguments, parser)),
        Argument(Short('V') | Long("version")) if arguments.command.is_none() => Some(self::parse_version()),

        Argument(Long("color")) => self::parse_color(arguments, parser),

        Argument(Short('a') | Long("all")) => self::parse_all(arguments),

        Argument(Short('e') | Long("exclude")) => self::parse_exclude(arguments, parser),
        Argument(Short('i') | Long("include")) => self::parse_include(arguments, parser),

        Argument(Short('r') | Long("resolve-symlinks")) => self::parse_resolve_symlinks(arguments),

        Argument(Long("sort")) => self::parse_sort(arguments, parser),

        Argument(Short('m') | Long("mode")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_mode(arguments, parser)
        }

        Argument(Short('s') | Long("size")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_size(arguments, parser)
        }

        Argument(Long("created")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_time(arguments, parser, TimeSectionType::Created)
        }
        Argument(Long("accessed")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_time(arguments, parser, TimeSectionType::Accessed)
        }
        Argument(Long("modified")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_time(arguments, parser, TimeSectionType::Modified)
        }

        Argument(Short('u') | Long("user")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_user(arguments)
        }
        Argument(Short('g') | Long("group")) if arguments.command.as_ref().is_some_and(SubCommand::is_list) => {
            self::parse_group(arguments)
        }

        Argument(Short('d') | Long("depth")) if arguments.command.as_ref().is_some_and(SubCommand::is_tree) => {
            self::parse_depth(arguments, parser)
        }

        Positional(value) => self::parse_positional(arguments, value),
        Argument(_) => Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("unexpected argument `{argument}`"))),
    }
}

/// Parses a single positional command-line argument.
fn parse_positional(arguments: &mut Arguments, value: &str) -> Option<ParseResult> {
    if arguments.command.is_some() {
        match Path::new(value).canonicalize().map(PathBuf::into_boxed_path) {
            Ok(path) => {
                if !arguments.paths.contains(&path) {
                    arguments.paths.push(path);
                }
            }
            Err(error) => return Some(self::exit_and_print(ERROR_GENERIC, error)),
        }
    } else {
        arguments.command = Some(match value {
            "list" => SubCommand::List(ListArguments::default()),
            "tree" => SubCommand::Tree(TreeArguments::default()),
            _ => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("unknown subcommand `{value}`"))),
        });
    }

    None
}

/// Parses the help command-line argument.
fn parse_help<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> ParseResult
where
    I: Iterator<Item = &'p str>,
{
    if let Ok(Some(value)) =
        arguments.command.is_none().then(|| parser.parse_next_assigned_value()).transpose().map(Option::flatten)
    {
        // Attempt to read the next argument as a subcommand.
        drop(self::parse_positional(arguments, value));
    }

    self::exit_and_print(SUCCESS, HELP)
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
    let Some(choice) = (match parser.parse_next_assigned_value() {
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
#[inline]
const fn parse_all(arguments: &mut Arguments) -> Option<ParseResult> {
    arguments.show_hidden = true;

    None
}

/// Parses the resolve-symlinks command-line argument.
#[inline]
const fn parse_resolve_symlinks(arguments: &mut Arguments) -> Option<ParseResult> {
    arguments.resolve_symlinks = true;

    None
}

/// Parses the sort command-line argument.
fn parse_sort<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(orderings) = (match parser.parse_next_assigned_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing sort order"));
    };

    let mut sort_order = None::<SortOrder>;

    for string in orderings.split(',') {
        let (string, is_reversed) = string.strip_prefix("reverse-").map_or((string, false), |string| (string, true));

        let Some(next) = (match string {
            "name" => Some(SortOrder::Name),
            "accessed" => Some(SortOrder::Accessed),
            "created" => Some(SortOrder::Created),
            "modified" => Some(SortOrder::Modified),
            "size" => Some(SortOrder::Size),
            "files" => Some(SortOrder::Files),
            "symlinks" => Some(SortOrder::Symlinks),
            "directories" => Some(SortOrder::Directories),
            "hidden" => Some(SortOrder::Hidden),
            _ => None,
        }) else {
            return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid sort order '{string}'")));
        };

        let next = if is_reversed { next.reverse() } else { next };

        if let Some(current) = sort_order.take().filter(|v| v.top() != &next) {
            sort_order = Some(current.then(next));
        } else {
            sort_order = Some(next);
        }
    }

    arguments.sort_order = sort_order;

    None
}

/// Parses the mode command-line argument.
fn parse_mode<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.parse_next_assigned_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing mode visibility"));
    };

    let Some(SubCommand::List(ListArguments { mode, .. })) = arguments.command.as_mut() else { unreachable!() };

    *mode = match choice {
        "hide" => None,
        "show" => Some(ModeSection::new(false)),
        "extended" => Some(ModeSection::new(true)),
        v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid mode visibility '{v}'"))),
    };

    None
}

/// Parses the size command-line argument.
fn parse_size<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.parse_next_assigned_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing size visibility"));
    };

    let Some(SubCommand::List(ListArguments { size, .. })) = arguments.command.as_mut() else { unreachable!() };

    *size = match choice {
        "hide" => None,
        "simple" => Some(SizeSection::new(SizeVisibility::Simple)),
        "base-2" => Some(SizeSection::new(SizeVisibility::Base2)),
        "base-10" => Some(SizeSection::new(SizeVisibility::Base10)),
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
    let Some(choice) = (match parser.parse_next_assigned_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing time visibility"));
    };

    let Some(SubCommand::List(ListArguments { created, accessed, modified, .. })) = arguments.command.as_mut() else {
        unreachable!();
    };

    *(match kind {
        TimeSectionType::Created => created,
        TimeSectionType::Accessed => accessed,
        TimeSectionType::Modified => modified,
    }) = match choice {
        "hide" => None,
        "simple" => Some(TimeSection::new(TimeVisibility::Simple, kind)),
        "iso8601" => Some(TimeSection::new(TimeVisibility::Iso8601, kind)),
        v => return Some(self::exit_and_print(ERROR_CLI_USAGE, format_args!("invalid time visibility '{v}'"))),
    };

    None
}

/// Parses the user command-line argument.
fn parse_user(arguments: &mut Arguments) -> Option<ParseResult> {
    let Some(command) = arguments.command.as_mut() else { unreachable!() };

    match command {
        SubCommand::List(arguments) => arguments.user = Some(UserSection),
        SubCommand::Tree(_) => unreachable!(),
    }

    None
}

/// Parses the group command-line argument.
fn parse_group(arguments: &mut Arguments) -> Option<ParseResult> {
    let Some(command) = arguments.command.as_mut() else { unreachable!() };

    match command {
        SubCommand::List(arguments) => arguments.group = Some(GroupSection),
        SubCommand::Tree(_) => unreachable!(),
    }

    None
}

/// Parses the exclude command-line argument.
fn parse_exclude<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(path) = (match parser.parse_next_assigned_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing excluded path"));
    };
    let path = match std::fs::canonicalize(path) {
        Ok(path) => path.into_boxed_path(),
        Err(error) => return Some(self::exit_and_print(ERROR_GENERIC, error)),
    };

    arguments.excluded.get_or_insert_default().insert(path);

    None
}

/// Parses the include command-line argument.
fn parse_include<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(path) = (match parser.parse_next_assigned_value() {
        Ok(choice) => choice,
        Err(error) => return Some(self::exit_and_print(ERROR_CLI_USAGE, error)),
    }) else {
        return Some(self::exit_and_print(ERROR_CLI_USAGE, "missing included path"));
    };
    let path = match std::fs::canonicalize(path) {
        Ok(path) => path.into_boxed_path(),
        Err(error) => return Some(self::exit_and_print(ERROR_GENERIC, error)),
    };

    arguments.included.get_or_insert_default().insert(path);

    None
}

/// Parses the depth command-line argument.
fn parse_depth<'p, I>(arguments: &mut Arguments, parser: &mut Parser<&'p str, I>) -> Option<ParseResult>
where
    I: Iterator<Item = &'p str>,
{
    let Some(choice) = (match parser.parse_next_assigned_value() {
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
