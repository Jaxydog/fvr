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

//! Defines structures that allow command-line options to be outlined easily.

use std::io::Write;

macro_rules! view_indexed {
    ($slice:expr, | _, $value_ident:ident | $($view_body:tt)+) => {
        view_indexed!($slice, |index, $value_ident| $($view_body)+)
    };
    ($slice:expr, | $index_ident:ident, $value_ident:ident | $($view_body:tt)+) => {
        view_indexed!($slice, 0, |$index_ident, $value_ident| $($view_body)+)
    };
    ($slice:expr, $start_index:expr, $message:literal, | _, $value_ident:ident | $($view_body:tt)+) => {
        view_indexed!($slice, $start_index, |index, $value_ident| $($view_body)+)
    };
    ($slice:expr, $start_index:expr, | $index_ident:ident, $value_ident:ident | $($view_body:tt)+) => {{
        let mut $index_ident: ::std::primitive::usize = $start_index;

        while $index_ident < $slice.len() {
            let $value_ident = $slice[$index_ident];

            { $($view_body)+ }

            $index_ident += 1;
        }
    }};
}

macro_rules! test_indexed {
    ($slice:expr, $message:literal, | _, $value_ident:ident | $($view_body:tt)+) => {
        test_indexed!($slice, $message, |index, $value_ident| $($view_body)+)
    };
    ($slice:expr, $message:literal, | $index_ident:ident, $value_ident:ident | $($view_body:tt)+) => {
        test_indexed!($slice, 0, $message, |$index_ident, $value_ident| $($view_body)+)
    };
    ($slice:expr, $start_index:expr, $message:literal, | _, $value_ident:ident | $($view_body:tt)+) => {
        test_indexed!($slice, $start_index, $message, |index, $value_ident| $($view_body)+)
    };
    ($slice:expr, $start_index:expr, $message:literal, | $index_ident:ident, $value_ident:ident | $($view_body:tt)+) => {
        view_indexed!($slice, $start_index, |$index_ident, $value_ident| ::std::assert!({ $($view_body)+ }, $message))
    };
}

/// Asserts that the given string:
///
/// - Is composed entire of valid ASCII characters.
/// - Is not bounded by excess whitespace characters.
/// - Is non-empty.
/// - Does not contain any control characters.
///
/// # Panics
///
/// This function will panic if any of the above conditions are violated.
#[inline]
const fn assert_ascii(string: &str) {
    assert!(string.is_ascii(), "string must be valid ascii");
    assert!(string.eq_ignore_ascii_case(string.trim_ascii()), "string must not contain excess whitespace");
    assert!(!string.is_empty(), "string must not be empty");

    test_indexed!(string.as_bytes(), "string must not contain control characters", |_, byte| !byte.is_ascii_control());
}

/// Writes the given command schema into the provided writer as a help display.
///
/// # Errors
///
/// This function will return an error if writing fails.
pub fn write_help(schema: CommandSchema<'_>, f: &mut impl Write) -> std::io::Result<()> {
    f.write_all(schema.name.as_bytes())?;

    if let Some(version) = schema.version {
        write!(f, " v{version}")?;
    }

    write!(f, "\n  {}\n\nUsage: {}", schema.about, schema.name)?;

    if schema.commands.is_some() {
        f.write_all(b" [SUBCOMMAND]")?;
    }
    if schema.arguments.is_some() {
        f.write_all(b" [ARGUMENTS]")?;
    }

    for ValueSchema { name, list, required, .. } in schema.positionals.into_iter().flat_map(|v| v.iter()) {
        write!(f, " [{name}")?;

        if *list {
            f.write_all(if *required { b"..." } else { b"..?" })?;
        } else if !*required {
            f.write_all(b"?")?;
        }

        f.write_all(b"]")?;
    }

    f.write_all(b"\n")?;

    if let Some(commands) = schema.commands {
        f.write_all(b"\nSub-commands:\n")?;

        for CommandSchema { name, about, .. } in commands {
            writeln!(f, "  {name: <30} {about}")?;
        }
    }

    if let Some(positionals) = schema.positionals {
        f.write_all(b"\nPositionals:\n")?;

        for ValueSchema { name, about, default, options, .. } in positionals {
            writeln!(f, "  {name: <30} {}", about.unwrap_or(""))?;

            if let Some(default) = default {
                writeln!(f, "{: <32} - default: {default}", "")?;
            }
            if let Some(options) = options {
                writeln!(f, "{: <32} - options: {}", "", options.join(", "))?;
            }
        }
    }

    if let Some(arguments) = schema.arguments {
        f.write_all(b"\nArguments:\n")?;

        for ArgumentSchema { long, short, about, value } in arguments {
            if let Some(short) = short {
                write!(f, "  -{short}, ")?;
            } else {
                write!(f, "      ")?;
            }

            if let Some(ValueSchema { name, list, required, .. }) = value {
                let mut temp = Vec::with_capacity(long.len() + 1 + name.len() + 6);

                write!(&mut temp, "{long} [{name}")?;

                if *list {
                    temp.write_all(if *required { b"..." } else { b"..?" })?;
                } else if !*required {
                    temp.write_all(b"?")?;
                }

                temp.write_all(b"]")?;

                write!(f, "--{: <24}", String::from_utf8_lossy(&temp))?;
            } else {
                write!(f, "--{long: <24}")?;
            }

            writeln!(f, " {about}")?;

            if let Some(ValueSchema { default, options, .. }) = value {
                if let Some(default) = default {
                    writeln!(f, "{: <32} - default: {default}", "")?;
                }
                if let Some(options) = options {
                    writeln!(f, "{: <32} - options: {}", "", options.join(", "))?;
                }
            }
        }
    }

    Ok(())
}

/// A command schema definition.
#[must_use = "schema definitions do nothing by themselves"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandSchema<'s> {
    /// The command name.
    pub name: &'s str,
    /// The command description.
    pub about: &'s str,
    /// The command version.
    pub version: Option<&'s str>,
    /// The command's arguments.
    pub arguments: Option<&'s [ArgumentSchema<'s>]>,
    /// The command's positional arguments.
    pub positionals: Option<&'s [ValueSchema<'s>]>,
    /// The command's sub-commands.
    pub commands: Option<&'s [Self]>,
}

impl CommandSchema<'_> {
    /// Asserts that this command is valid and may be used within a command-line application.
    ///
    /// # Panics
    ///
    /// Panics if this schema is not valid.
    pub const fn validate(self) -> Self {
        self::assert_ascii(self.name);

        test_indexed!(self.name.as_bytes(), "command name should be entirely lowercase", |_, byte| {
            !byte.is_ascii_alphabetic() || byte.is_ascii_lowercase()
        });

        self::assert_ascii(self.about);

        if let Some(version) = self.version {
            self::assert_ascii(version);
        }

        if let Some(arguments) = self.arguments {
            assert!(!arguments.is_empty(), "at least one argument should be provided");

            view_indexed!(arguments, |index, argument| {
                _ = argument.validate();

                test_indexed!(
                    arguments,
                    index + 1,
                    "command arguments should not contain duplicate values",
                    |_, other| argument.const_ne(other)
                );
            });
        }

        if let Some(positionals) = self.positionals {
            assert!(!positionals.is_empty(), "at least one positional should be provided");

            view_indexed!(positionals, |index, positional| {
                _ = positional.validate();

                test_indexed!(
                    positionals,
                    index + 1,
                    "command positionals should not contain duplicates",
                    |_, other| !positional.name.eq_ignore_ascii_case(other.name)
                );
            });
        }

        if let Some(commands) = self.commands {
            assert!(!commands.is_empty(), "at least one command should be provided");

            view_indexed!(commands, |index, command| {
                _ = command.validate();

                test_indexed!(commands, index + 1, "sub-commands should not contain duplicates", |_, other| {
                    !command.name.eq_ignore_ascii_case(other.name)
                });
            });
        }

        self
    }
}

/// A command schema definition builder.
#[repr(transparent)]
#[must_use = "the build function must be invoked"]
pub struct CommandSchemaBuilder<'s> {
    inner: CommandSchema<'s>,
}

impl<'s> CommandSchemaBuilder<'s> {
    /// Creates a new [`CommandSchemaBuilder`].
    pub const fn new(name: &'s str, about: &'s str) -> Self {
        Self { inner: CommandSchema { name, about, version: None, arguments: None, positionals: None, commands: None } }
    }

    /// Sets the command version.
    pub const fn version(mut self, version: &'s str) -> Self {
        self.inner.version = Some(version);

        self
    }

    /// Sets the command's positional arguments.
    pub const fn positionals(mut self, positionals: &'s [ValueSchema<'s>]) -> Self {
        self.inner.positionals = Some(positionals);

        self
    }

    /// Sets the command's arguments.
    pub const fn arguments(mut self, arguments: &'s [ArgumentSchema<'s>]) -> Self {
        self.inner.arguments = Some(arguments);

        self
    }

    /// Sets the command's sub-commands.
    pub const fn commands(mut self, commands: &'s [CommandSchema<'s>]) -> Self {
        self.inner.commands = Some(commands);

        self
    }

    /// Builds and validates the schema.
    pub const fn build(self) -> CommandSchema<'s> {
        self.inner.validate()
    }
}

/// A command value schema definition.
#[must_use = "schema definitions do nothing by themselves"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueSchema<'s> {
    /// The value name.
    pub name: &'s str,
    /// The value description.
    pub about: Option<&'s str>,
    /// Whether more than one value is supported.
    pub list: bool,
    /// Whether this value is required.
    pub required: bool,
    /// The default value string if not provided.
    pub default: Option<&'s str>,
    /// The allowed value strings.
    pub options: Option<&'s [&'s str]>,
}

impl ValueSchema<'_> {
    /// Asserts that this command value is valid and may be used within a command-line application.
    ///
    /// # Panics
    ///
    /// Panics if this schema is not valid.
    pub const fn validate(self) -> Self {
        self::assert_ascii(self.name);

        test_indexed!(self.name.as_bytes(), "value name should be entirely uppercase", |_, byte| {
            !byte.is_ascii_alphabetic() || byte.is_ascii_uppercase()
        });

        if let Some(about) = self.about {
            self::assert_ascii(about);
        }

        if let Some(options) = self.options {
            assert!(!options.is_empty(), "at least one option should be provided");

            if !self.list
                && let Some(default) = self.default
            {
                let mut contained = false;

                view_indexed!(options, |_, option| contained |= option.eq_ignore_ascii_case(default));

                assert!(contained, "default value should be contained within the options array");
            }

            view_indexed!(options, |index, option| {
                self::assert_ascii(option);

                test_indexed!(option.as_bytes(), "option should be entirely lowercase", |_, byte| {
                    !byte.is_ascii_alphabetic() || byte.is_ascii_lowercase()
                });
                test_indexed!(options, index + 1, "options should not contain duplicates", |_, other| {
                    !option.eq_ignore_ascii_case(other)
                });
            });
        }

        self
    }
}

/// A value schema definition builder.
#[repr(transparent)]
#[must_use = "the build function must be invoked"]
pub struct ValueSchemaBuilder<'s> {
    inner: ValueSchema<'s>,
}

impl<'s> ValueSchemaBuilder<'s> {
    /// Creates a new [`ValueSchemaBuilder`].
    pub const fn new(name: &'s str) -> Self {
        Self { inner: ValueSchema { name, about: None, list: false, required: false, default: None, options: None } }
    }

    /// Sets the value's description.
    pub const fn about(mut self, about: &'s str) -> Self {
        self.inner.about = Some(about);

        self
    }

    /// Sets the value to be a list of values.
    pub const fn list(mut self) -> Self {
        self.inner.list = true;

        self
    }

    /// Sets the value to be required.
    pub const fn required(mut self) -> Self {
        self.inner.required = true;

        self
    }

    /// Sets the value's default value.
    pub const fn default(mut self, default: &'s str) -> Self {
        self.inner.default = Some(default);

        self
    }

    /// Sets the value's options.
    pub const fn options(mut self, options: &'s [&'s str]) -> Self {
        self.inner.options = Some(options);

        self
    }

    /// Builds and validates the schema.
    pub const fn build(self) -> ValueSchema<'s> {
        self.inner.validate()
    }
}

/// A command argument schema definition.
#[must_use = "schema definitions do nothing by themselves"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgumentSchema<'s> {
    /// The long argument name.
    pub long: &'s str,
    /// The short argument name.
    pub short: Option<char>,
    /// The argument description.
    pub about: &'s str,
    /// The argument value name.
    pub value: Option<ValueSchema<'s>>,
}

impl ArgumentSchema<'_> {
    /// Returns `true` if this argument is **not** equal to the given argument.
    const fn const_ne(self, other: Self) -> bool {
        if let Some(self_short) = self.short
            && let Some(other_short) = other.short
            && self_short == other_short
        {
            return false;
        }

        !self.long.eq_ignore_ascii_case(other.long)
    }

    /// Asserts that this command argument is valid and may be used within a command-line application.
    ///
    /// # Panics
    ///
    /// Panics if this schema is not valid.
    pub const fn validate(self) -> Self {
        self::assert_ascii(self.long);

        test_indexed!(self.long.as_bytes(), "long argument name should be entirely lowercase", |_, byte| {
            !byte.is_ascii_alphabetic() || byte.is_ascii_lowercase()
        });

        self::assert_ascii(self.about);

        if let Some(short) = self.short {
            assert!(short.is_ascii_alphanumeric(), "short argument name should be alphanumeric");
        }

        if let Some(value) = self.value {
            _ = value.validate();
        }

        self
    }
}

/// An argument schema definition builder.
#[repr(transparent)]
#[must_use = "the build function must be invoked"]
pub struct ArgumentSchemaBuilder<'s> {
    inner: ArgumentSchema<'s>,
}

impl<'s> ArgumentSchemaBuilder<'s> {
    /// Creates a new [`ArgumentSchemaBuilder`].
    pub const fn new(long: &'s str, about: &'s str) -> Self {
        Self { inner: ArgumentSchema { long, short: None, about, value: None } }
    }

    /// Sets the argument's short name.
    pub const fn short(mut self, short: char) -> Self {
        self.inner.short = Some(short);

        self
    }

    /// Sets the argument's value.
    pub const fn value(mut self, value: ValueSchema<'s>) -> Self {
        self.inner.value = Some(value);

        self
    }

    /// Builds and validates the schema.
    pub const fn build(self) -> ArgumentSchema<'s> {
        self.inner.validate()
    }
}
