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

//! Provides composable filtering types.

use std::fs::Metadata;
use std::path::Path;

/// Returns a [`Filter`] that allows all entries.
#[inline]
#[must_use]
pub const fn all() -> By<impl Fn(&Path, &Metadata) -> bool> {
    self::by(|_, _| true)
}

/// Returns a [`Filter`] that disallows all entries.
#[inline]
#[must_use]
pub const fn none() -> By<impl Fn(&Path, &Metadata) -> bool> {
    self::by(|_, _| false)
}

/// Returns a [`Filter`] that allows entries that match a custom predicate.
#[inline]
pub const fn by<F>(f: F) -> By<F>
where
    F: Fn(&Path, &Metadata) -> bool,
{
    By(f)
}

/// Returns a [`Filter`] that allows entries that match a custom predicate, account for depth.
#[inline]
pub const fn depth_by<F>(f: F) -> DepthBy<F>
where
    F: Fn(&Path, &Metadata, usize) -> bool,
{
    DepthBy(f)
}

/// A value that can be used to filter out entries from a visit call.
#[must_use = "filters do nothing unless provided to a visit call"]
pub trait Filter: Sized {
    /// Returns `true` if the entry should be retained.
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool;

    /// Returns `true` if the entry should be retained, accounting for depth.
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, depth: usize) -> bool;

    /// Returns a new [`Filter`] that returns the inverse of this [`Filter`].
    #[inline]
    fn not(self) -> Not<Self> {
        Not(self)
    }

    /// Returns a new [`Filter`] that returns the logical 'and' of this [`Filter`] and the provided [`Filter`].
    #[inline]
    fn and<U: Filter>(self, other: U) -> And<Self, U> {
        And(self, other)
    }

    /// Returns a new [`Filter`] that returns the logical 'or' of this [`Filter`] and the provided [`Filter`].
    #[inline]
    fn or<U: Filter>(self, other: U) -> Or<Self, U> {
        Or(self, other)
    }

    /// Returns a new [`Filter`] that returns the logical 'exclusive-or' of this [`Filter`] and the provided [`Filter`].
    #[inline]
    fn xor<U: Filter>(self, other: U) -> Xor<Self, U> {
        Xor(self, other)
    }
}

/// Allows entries that match a custom predicate.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct By<F>(F);

impl<F> Filter for By<F>
where
    F: Fn(&Path, &Metadata) -> bool,
{
    #[inline]
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool {
        (self.0)(path, data)
    }

    #[inline]
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, _: usize) -> bool {
        self.filter(path, data)
    }
}

/// Allows entries that match a custom predicate, accounting for depth.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DepthBy<F>(F);

impl<F> Filter for DepthBy<F>
where
    F: Fn(&Path, &Metadata, usize) -> bool,
{
    #[inline]
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool {
        self.depth_filter(path, data, 0)
    }

    #[inline]
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, depth: usize) -> bool {
        (self.0)(path, data, depth)
    }
}

/// Allow the inverse of the inner [`Filter`].
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Not<T>(T);

impl<T: Filter> Filter for Not<T> {
    #[inline]
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool {
        !self.0.filter(path, data)
    }

    #[inline]
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, depth: usize) -> bool {
        !self.0.depth_filter(path, data, depth)
    }
}

/// Returns the logical 'and' of the inner [`Filter`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct And<T, U>(T, U);

impl<T: Filter, U: Filter> Filter for And<T, U> {
    #[inline]
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool {
        self.0.filter(path, data) && self.1.filter(path, data)
    }

    #[inline]
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, depth: usize) -> bool {
        self.0.depth_filter(path, data, depth) && self.1.depth_filter(path, data, depth)
    }
}

/// Returns the logical 'or' of the inner [`Filter`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Or<T, U>(T, U);

impl<T: Filter, U: Filter> Filter for Or<T, U> {
    #[inline]
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool {
        self.0.filter(path, data) || self.1.filter(path, data)
    }

    #[inline]
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, depth: usize) -> bool {
        self.0.depth_filter(path, data, depth) || self.1.depth_filter(path, data, depth)
    }
}

/// Returns the logical 'exclusive-or' of the inner [`Filter`]s.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Xor<T, U>(T, U);

impl<T: Filter, U: Filter> Filter for Xor<T, U> {
    #[inline]
    fn filter<'p>(&self, path: &'p Path, data: &'p Metadata) -> bool {
        self.0.filter(path, data) ^ self.1.filter(path, data)
    }

    #[inline]
    fn depth_filter<'p>(&self, path: &'p Path, data: &'p Metadata, depth: usize) -> bool {
        self.0.depth_filter(path, data, depth) ^ self.1.depth_filter(path, data, depth)
    }
}
