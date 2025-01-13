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

//! Implements command-line argument parsing.
//!
//! Takes heavy inspiration from the [`getargs`][0] crate.
//!
//! [0]: https://github.com/j-tai/getargs

use std::ffi::OsStr;
use std::fmt::{Debug, Display, Formatter};

/// An error that may be returned while parsing command-line arguments.
#[derive(Debug, thiserror::Error)]
pub enum Error<A: ArgumentLike> {
    /// Returned by the parser if a cluster does not contain any arguments.
    #[error("an empty short-form argument cluster was encountered")]
    EmptyCluster,
    /// Returned by the parser if a value is attempted to be retrieved with no prior argument.
    #[error("attempted to retrieve value with no previous argument")]
    MissingAssignedArgument,
    /// Returned by the parser if a non-positional argument was ignored.
    #[error("a non-positional argument was skipped")]
    SkippedNonPositional,
    /// Returned by the parser if an argument's value was ignored.
    #[error("the value for {0} was skipped")]
    SkippedValue(Argument<A>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParserState<A: ArgumentLike> {
    Start { arguments_closed: bool },
    Ready(Argument<A>),
    HasValue(Argument<A>, A),
    HasCluster(Argument<A>, A),
    HasPositional(A),
    Finished { arguments_closed: bool },
}

/// A command-line option parser.
#[derive(Clone, Copy, Debug)]
pub struct Parser<A: ArgumentLike, I> {
    /// The parser's internal argument iterator.
    iterator: I,
    /// Retains the parser's state across `next_*` calls.
    state: ParserState<A>,
}

impl<A: ArgumentLike, I: Iterator<Item = A>> Parser<A, I> {
    /// Creates a new [`Parser<A, I>`].
    pub const fn new(iterator: I) -> Self {
        Self { iterator, state: ParserState::Start { arguments_closed: false } }
    }

    /// Returns the next argument of this [`Parser<A, I>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an argument's assigned value is ignored.
    pub fn next_argument(&mut self) -> Result<Option<Argument<A>>, Error<A>> {
        if !matches!(
            self.state,
            ParserState::Start { arguments_closed: true } | ParserState::Finished { arguments_closed: true }
        ) {
            if let Ok(Some(argument)) = self.next_parameter() {
                return Ok(Some(argument.into()));
            }
        }

        self.next_positional().map(|v| v.map(Argument::Positional))
    }

    /// Returns the next positional argument of this [`Parser<A, I>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if a non-positional argument was skipped.
    pub fn next_positional(&mut self) -> Result<Option<A>, Error<A>> {
        match self.state {
            ParserState::Start { arguments_closed } => Ok(self.iterator.next().or_else(|| {
                self.state = ParserState::Finished { arguments_closed };

                None
            })),
            ParserState::HasPositional(argument) => {
                self.state = ParserState::Start { arguments_closed: false };

                Ok(Some(argument))
            }
            ParserState::Finished { .. } => Ok(None),
            _ => Err(Error::SkippedNonPositional),
        }
    }

    /// Returns the next non-positional argument of this [`Parser<A, I>`].
    ///
    /// If you want to receive positional arguments in-between non-positional arguments, use [`next_argument`].
    ///
    /// # Errors
    ///
    /// This function will return an error if an argument's assigned value is ignored.
    ///
    /// [`next_argument`]: crate::arguments::parse::Parser::next_argument
    pub fn next_parameter(&mut self) -> Result<Option<Parameter<A>>, Error<A>> {
        match self.state {
            ParserState::Start { .. } | ParserState::Ready(_) => {
                let Some(argument) = self.iterator.next() else {
                    self.state = ParserState::Finished { arguments_closed: false };

                    return Ok(None);
                };

                if argument.is_closing_argument() {
                    self.state = ParserState::Start { arguments_closed: true };

                    Ok(None)
                } else if let Some((argument, value)) = argument.as_long_argument() {
                    let argument = Parameter::Long(argument);

                    self.state = value.map_or_else(
                        || ParserState::Ready(argument.into()),
                        |v| ParserState::HasValue(argument.into(), v),
                    );

                    Ok(Some(argument))
                } else if let Some(cluster) = argument.as_short_cluster() {
                    let Some((argument, cluster)) = cluster.as_short_argument() else {
                        self.state = ParserState::Start { arguments_closed: false };

                        return Err(Error::EmptyCluster);
                    };

                    let argument = Parameter::Short(argument);

                    self.state = cluster.map_or_else(
                        || ParserState::Ready(argument.into()),
                        |v| ParserState::HasCluster(argument.into(), v),
                    );

                    Ok(Some(argument))
                } else {
                    self.state = ParserState::HasPositional(argument);

                    Ok(None)
                }
            }
            ParserState::HasCluster(_, cluster) => {
                let Some((argument, cluster)) = cluster.as_short_argument() else {
                    self.state = ParserState::Start { arguments_closed: false };

                    return Err(Error::EmptyCluster);
                };

                let argument = Parameter::Short(argument);

                self.state = cluster.map_or_else(
                    || ParserState::Ready(argument.into()),
                    |v| ParserState::HasCluster(argument.into(), v),
                );

                Ok(Some(argument))
            }
            ParserState::HasValue(argument, _) => {
                self.state = ParserState::Start { arguments_closed: false };

                Err(Error::SkippedValue(argument))
            }
            ParserState::HasPositional(_) | ParserState::Finished { .. } => Ok(None),
        }
    }

    /// Returns the next value of this [`Parser<A, I>`].
    ///
    /// # Errors
    ///
    /// This function will return an error if this is called without first retrieving an associated argument.
    pub fn next_value(&mut self) -> Result<Option<A>, Error<A>> {
        match self.state {
            ParserState::HasValue(_, value) | ParserState::HasCluster(_, value) => {
                self.state = ParserState::Start { arguments_closed: false };

                Ok(Some(value))
            }
            ParserState::Ready(_) => {
                let value = self.iterator.next();

                self.state = if value.is_some() {
                    ParserState::Start { arguments_closed: false }
                } else {
                    ParserState::Finished { arguments_closed: false }
                };

                Ok(value)
            }
            ParserState::Start { .. } | ParserState::HasPositional(_) | ParserState::Finished { .. } => {
                Err(Error::MissingAssignedArgument)
            }
        }
    }
}

/// A non-positional argument returned by a [`Parser<A, I>`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter<A: ArgumentLike> {
    /// A short-form argument, such as `-h` or `-V`.
    Short(A::Short),
    /// A long-form argument, such as `--help` or `--color`.
    Long(A),
}

impl<A: ArgumentLike<Short: Display> + Display> Display for Parameter<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Short(v) => write!(f, "-{v}"),
            Self::Long(v) => write!(f, "--{v}"),
        }
    }
}

/// An argument returned by a [`Parser<A, I>`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Argument<A: ArgumentLike> {
    /// A short-form argument, such as `-h` or `-V`.
    Short(A::Short),
    /// A long-form argument, such as `--help` or `--color`.
    Long(A),
    /// A positional argument, such as `./path/to/file` or `any-other-string`.
    Positional(A),
}

impl<A: ArgumentLike> From<Parameter<A>> for Argument<A> {
    #[inline]
    fn from(value: Parameter<A>) -> Self {
        match value {
            Parameter::Short(v) => Self::Short(v),
            Parameter::Long(v) => Self::Long(v),
        }
    }
}

impl<A: ArgumentLike<Short: Display> + Display> Display for Argument<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Short(v) => write!(f, "-{v}"),
            Self::Long(v) => write!(f, "--{v}"),
            Self::Positional(v) => write!(f, "{v}"),
        }
    }
}

/// A value that can be parsed as an argument by a [`Parser<A, I>`].
pub trait ArgumentLike: Copy + Debug + Eq {
    /// The type used to represent a short argument (i.e., `-h`).
    type Short: Copy + Debug + Eq;

    /// Returns `true` if this value represents a "closing argument".
    ///
    /// For example, if the value `--` is passed, all further arguments are interpreted as positional.
    fn is_closing_argument(self) -> bool;

    /// Returns this value as a long-form argument if applicable (i.e., `--help` or `--color=always`).
    fn as_long_argument(self) -> Option<(Self, Option<Self>)>;

    /// Returns this value as a short-form cluster if applicable (i.e., `-h` or `-abcd`).
    fn as_short_cluster(self) -> Option<Self>;

    /// Returns a short-form argument from this value, assuming that it is a cluster.
    ///
    /// This should only be called on the value returned by [`as_short_cluster`].
    ///
    /// [`as_short_cluster`]: crate::arguments::parse::ArgumentLike::as_short_cluster
    fn as_short_argument(self) -> Option<(Self::Short, Option<Self>)>;
}

impl ArgumentLike for &'_ str {
    type Short = char;

    #[inline]
    fn is_closing_argument(self) -> bool {
        self == "--"
    }

    #[inline]
    fn as_long_argument(self) -> Option<(Self, Option<Self>)> {
        self.strip_prefix("--")
            .filter(|s| !s.is_empty())
            .map(|s| s.split_once('=').map_or((s, None), |(s, v)| (s, Some(v))))
    }

    #[inline]
    fn as_short_cluster(self) -> Option<Self> {
        self.strip_prefix('-').filter(|s| !s.is_empty() && !s.starts_with('-'))
    }

    #[inline]
    fn as_short_argument(self) -> Option<(Self::Short, Option<Self>)> {
        self.chars().next().map(|c| (c, Some(&self[c.len_utf8() ..]).filter(|s| !s.is_empty())))
    }
}

impl<'s> ArgumentLike for &'s [u8] {
    type Short = &'s [u8];

    fn is_closing_argument(self) -> bool {
        self == b"--"
    }

    fn as_long_argument(self) -> Option<(Self, Option<Self>)> {
        self.strip_prefix(b"--")
            .filter(|s| !s.is_empty())
            .map(|s| s.split_once(|b| *b == b'=').map_or((s, None), |(s, v)| (s, Some(v))))
    }

    fn as_short_cluster(self) -> Option<Self> {
        self.strip_prefix(b"-").filter(|s| !s.is_empty() && !s.starts_with(b"-"))
    }

    fn as_short_argument(self) -> Option<(Self::Short, Option<Self>)> {
        self.utf8_chunks().next().map(|c| {
            let len = c.valid().len() + c.invalid().len();

            (&self[0 .. len], Some(&self[len ..]).filter(|s| !s.is_empty()))
        })
    }
}

#[expect(unsafe_code, reason = "conversion to `OsStr` requires unsafe code")]
impl<'s> ArgumentLike for &'s OsStr {
    type Short = &'s [u8];

    fn is_closing_argument(self) -> bool {
        self.as_encoded_bytes().is_closing_argument()
    }

    fn as_long_argument(self) -> Option<(Self, Option<Self>)> {
        // Safety: the implementation of `ArgumentLike` for `&[u8]` properly retains provided ASCII and UTF-8 sequences.
        self.as_encoded_bytes().as_long_argument().map(|(a, v)| unsafe {
            (OsStr::from_encoded_bytes_unchecked(a), v.map(|v| OsStr::from_encoded_bytes_unchecked(v)))
        })
    }

    fn as_short_cluster(self) -> Option<Self> {
        // Safety: the implementation of `ArgumentLike` for `&[u8]` properly retains provided ASCII and UTF-8 sequences.
        self.as_encoded_bytes().as_short_cluster().map(|v| unsafe { OsStr::from_encoded_bytes_unchecked(v) })
    }

    fn as_short_argument(self) -> Option<(Self::Short, Option<Self>)> {
        self.as_encoded_bytes().as_short_argument().map(|(a, v)| {
            // Safety: the implementation of `ArgumentLike` for `&[u8]` properly retains provided ASCII and UTF-8
            // sequences.
            (a, unsafe { v.map(|v| OsStr::from_encoded_bytes_unchecked(v)) })
        })
    }
}
