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

/// Defines a command outline.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command<'s> {
    /// The command's name.
    pub name: &'s str,
    /// The command's description.
    pub about: &'s str,
    /// The command's version.
    pub version: Option<&'s str>,
    /// The command's positional arguments.
    pub positionals: Option<&'s [Value<'s>]>,
    /// The command's arguments.
    pub arguments: Option<&'s [Argument<'s>]>,
    /// The command's sub-commands.
    pub sub_commands: Option<&'s [Command<'s>]>,
}

impl<'s> Command<'s> {
    /// Creates a new [`Command`].
    pub const fn new(name: &'s str, about: &'s str) -> Self {
        Self { name, about, version: None, positionals: None, arguments: None, sub_commands: None }
    }

    /// Sets this command's version.
    pub const fn version(mut self, version: &'s str) -> Self {
        self.version = Some(version);

        self
    }

    /// Sets this command's positional arguments.
    pub const fn positionals(mut self, positionals: &'s [Value<'s>]) -> Self {
        self.positionals = Some(positionals);

        self
    }

    /// Sets this command's arguments.
    pub const fn arguments(mut self, arguments: &'s [Argument<'s>]) -> Self {
        self.arguments = Some(arguments);

        self
    }

    /// Sets this command's sub-commands.
    pub const fn sub_commands(mut self, sub_commands: &'s [Self]) -> Self {
        self.sub_commands = Some(sub_commands);

        self
    }

    /// Writes the command's outline to the given writer.
    ///
    /// # Errors
    ///
    /// This function will return an error if the outline could not be written.
    pub fn write_to(self, f: &mut impl Write) -> std::io::Result<()> {
        let Self { name, about, version, positionals, arguments, sub_commands } = self;

        f.write_all(name.as_bytes())?;

        if let Some(version) = version {
            write!(f, " v{version}")?;
        }

        write!(f, "\n  {about}\n\nUsage: {name}")?;

        if sub_commands.is_some() {
            f.write_all(b" [SUBCOMMAND]")?;
        }
        if arguments.is_some() {
            f.write_all(b" [ARGUMENTS]")?;
        }

        for Value { name, list, required, .. } in positionals.into_iter().flat_map(|v| v.iter()) {
            f.write_all(b" ")?;

            write!(f, "[{name}")?;

            if *list {
                f.write_all(if *required { b"..." } else { b"..?" })?;
            } else if !required {
                f.write_all(b"?")?;
            }

            f.write_all(b"]")?;
        }

        f.write_all(b"\n")?;

        if let Some(sub_commands) = sub_commands {
            f.write_all(b"\nSub-commands:\n")?;

            for Command { name, about, .. } in sub_commands {
                writeln!(f, "  {name: <30} {about}")?;
            }
        }

        if let Some(positionals) = positionals {
            f.write_all(b"\nPositionals:\n")?;

            for Value { name, about, default, options, .. } in positionals {
                write!(f, "  {name: <30}")?;

                if let Some(about) = about {
                    writeln!(f, " {about}")?;
                } else {
                    f.write_all(b"\n")?;
                }

                if let Some(default) = default {
                    writeln!(f, "{: <32} - default: {default}", "")?;
                }
                if let Some(options) = options {
                    writeln!(f, "{: <32} - options: {}", "", options.join(", "))?;
                }
            }
        }

        if let Some(arguments) = arguments {
            f.write_all(b"\nArguments:\n")?;

            for Argument { long, short, about, value } in arguments {
                if let Some(short) = short {
                    write!(f, "  -{short}, ")?;
                } else {
                    write!(f, "{: <6}", "")?;
                }

                if let Some(Value { name, list, required, .. }) = value {
                    let mut f2 = Vec::new();

                    write!(f2, "[{name}")?;

                    if *list {
                        f2.write_all(if *required { b"..." } else { b"..?" })?;
                    } else if !required {
                        f2.write_all(b"?")?;
                    }

                    f2.write_all(b"]")?;

                    write!(f, "--{: <24}", format!("{long} {}", String::from_utf8_lossy(&f2)))?;
                } else {
                    write!(f, "--{long: <24}")?;
                }

                writeln!(f, " {about}")?;

                if let Some(Value { default, options, .. }) = value {
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

    /// Asserts that this command is valid and may be used within a command-line application.
    ///
    /// # Panics
    ///
    /// Panics if this schema is not valid.
    pub fn validate(self) {
        let Self { name, about, version, positionals, arguments, sub_commands } = self;

        self::validate_string(name, true);
        self::validate_string_is_lowercase(name);

        assert!(!about.is_empty(), "{name:?}'s description should not be empty");
        assert!(!about.contains(char::is_control), "{name:?}'s description should not contain a control character");

        version.inspect(|v| self::validate_string(v, false));

        if let Some(positionals) = positionals {
            assert!(!positionals.is_empty(), "{name:?}'s positionals should be `None` if the list is empty");

            positionals.iter().copied().for_each(Value::validate);

            let mut deduped: Vec<_> = positionals.iter().map(|v| v.name).collect();

            deduped.dedup();

            assert_eq!(positionals.len(), deduped.len(), "{name:?} should not contain duplicate positionals");
        }

        if let Some(arguments) = arguments {
            assert!(!arguments.is_empty(), "{name:?}'s arguments should be `None` if the list is empty");

            arguments.iter().copied().for_each(Argument::validate);

            let mut deduped: Vec<_> = arguments.iter().map(|v| v.long).collect();

            deduped.dedup();

            assert_eq!(arguments.len(), deduped.len(), "{name:?} should not contain duplicate long arguments");

            let mut deduped: Vec<_> = arguments.iter().filter_map(|v| v.short).collect();

            deduped.dedup();

            assert_eq!(
                arguments.iter().filter_map(|v| v.short).count(),
                deduped.len(),
                "{name:?} should not contain duplicate short arguments"
            );
        }

        if let Some(sub_commands) = sub_commands {
            assert!(!sub_commands.is_empty(), "{name:?}'s sub-commands should be `None` if the list is empty");

            for sub_command in sub_commands {
                assert!(sub_command.version.is_none(), "{:?} should not contain a version", sub_command.name);

                sub_command.validate();
            }

            let mut deduped: Vec<_> = sub_commands.iter().map(|v| v.name).collect();

            deduped.dedup();

            assert_eq!(sub_commands.len(), deduped.len(), "{name:?} should not contain duplicate sub-commands");
        }
    }
}

/// Defines an argument outline.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argument<'s> {
    /// The argument's long name.
    pub long: &'s str,
    /// The argument's short name.
    pub short: Option<char>,
    /// The argument's description.
    pub about: &'s str,
    /// The argument's value name.
    pub value: Option<Value<'s>>,
}

impl<'s> Argument<'s> {
    /// Creates a new [`Argument`].
    pub const fn new(long: &'s str, about: &'s str) -> Self {
        Self { long, short: None, about, value: None }
    }

    /// Sets this argument's short name.
    pub const fn short(mut self, short: char) -> Self {
        self.short = Some(short);

        self
    }

    /// Sets this argument's value.
    pub const fn value(mut self, value: Value<'s>) -> Self {
        self.value = Some(value);

        self
    }

    /// Asserts that this argument is valid and may be used within a command-line application.
    ///
    /// # Panics
    ///
    /// Panics if this schema is not valid.
    pub fn validate(self) {
        let Self { long, short, about, value } = self;

        self::validate_string(long, true);
        self::validate_string_is_lowercase(long);

        assert!(!about.is_empty(), "{long:?}'s description should not be empty");
        assert!(!about.contains(char::is_control), "{long:?}'s description should not contain a control character");

        if let Some(short) = short {
            assert!(short.is_ascii_alphanumeric(), "{short:?} should be ascii-alphanumeric");
        }

        value.inspect(|v| Value::validate(*v));
    }
}

/// Defines a value outline.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value<'s> {
    /// The value's name.
    pub name: &'s str,
    /// The value's description.
    pub about: Option<&'s str>,
    /// Whether more than one value is supported.
    pub list: bool,
    /// Whether this option is required.
    pub required: bool,
    /// The default value if not provided.
    pub default: Option<&'s str>,
    /// The allowed values.
    pub options: Option<&'s [&'s str]>,
}

impl<'s> Value<'s> {
    /// Creates a new [`Value`].
    pub const fn new(name: &'s str) -> Self {
        Self { name, about: None, list: false, required: false, default: None, options: None }
    }

    /// Sets this value's description.
    pub const fn about(mut self, about: &'s str) -> Self {
        self.about = Some(about);

        self
    }

    /// Sets this value as a list of values.
    pub const fn list(mut self) -> Self {
        self.list = true;

        self
    }

    /// Sets this value as being required.
    pub const fn required(mut self) -> Self {
        self.required = true;

        self
    }

    /// Sets this value's default value.
    pub const fn default(mut self, default: &'s str) -> Self {
        self.default = Some(default);

        self
    }

    /// Sets this value's options.
    pub const fn options(mut self, options: &'s [&'s str]) -> Self {
        self.options = Some(options);

        self
    }

    /// Asserts that this value is valid and may be used within a command-line application.
    ///
    /// # Panics
    ///
    /// Panics if this schema is not valid.
    pub fn validate(Self { name, about, default, options, .. }: Self) {
        self::validate_string(name, true);
        self::validate_string_is_uppercase(name);

        if let Some(about) = about {
            assert!(!about.is_empty(), "{name:?}'s description should not be empty");
            assert!(!about.contains(char::is_control), "{name:?}'s description should not contain a control character");
        }

        if let Some(default) = default {
            self::validate_string(default, true);
        }

        if let Some(options) = options {
            assert!(!options.is_empty(), "{name:?}'s options should be `None` if the list is empty");

            for option in options {
                self::validate_string(option, true);
                self::validate_string_is_lowercase(option);
            }

            let mut deduped = options.to_vec();

            deduped.dedup();

            assert_eq!(options.len(), deduped.len(), "{name:?} should not contain duplicate options");

            if let Some(default) = default {
                assert!(options.contains(&default), "{name:?}'s options should contain the set default");
            }
        }
    }
}

#[inline]
fn validate_string(string: &str, require_ascii: bool) {
    assert!(!string.is_empty(), "{string:?} should not be empty");
    assert!(!string.contains(char::is_whitespace), "{string:?} should not contain whitespace");
    assert!(!string.contains(char::is_control), "{string:?} should not contain any control characters");

    if require_ascii {
        assert!(string.is_ascii(), "{string:?} should be ascii-alphanumeric");
    }
}

#[inline]
fn validate_string_is_uppercase(string: &str) {
    assert!(!string.contains(char::is_lowercase), "{string:?} should only contain uppercase letters or symbols");
}

#[inline]
fn validate_string_is_lowercase(string: &str) {
    assert!(!string.contains(char::is_uppercase), "{string:?} should only contain lowercase letters or symbols");
}
