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

//! Provides composable sorting types.

use std::cmp::Ordering;
use std::fs::Metadata;
use std::marker::PhantomData;
use std::ops::Try;
use std::path::Path;

/// Returns a [`Sorter`] in which all entries are considered equal.
#[inline]
#[must_use]
#[expect(clippy::type_complexity, reason = "no reason to abstract this out")]
pub const fn equal() -> By<impl Fn((&Path, &Metadata), (&Path, &Metadata)) -> Ordering> {
    self::by(|_, _| Ordering::Equal)
}

/// Returns a [`Sorter`] that sorts entries based on the given sort function.
#[inline]
pub const fn by<F>(f: F) -> By<F>
where
    F: Fn((&Path, &Metadata), (&Path, &Metadata)) -> Ordering,
{
    By(f)
}

/// Returns a [`Sorter`] that sorts entries based on the given sort function, accounting for depth.
#[inline]
pub const fn depth_by<F>(f: F) -> DepthBy<F>
where
    F: Fn((&Path, &Metadata), (&Path, &Metadata), usize) -> Ordering,
{
    DepthBy(f)
}

/// Returns a [`Sorter`] that sorts entries based on the [`Ord`] implementation of the extracted value.
#[inline]
pub const fn extract<F, T>(f: F) -> Extract<F, T>
where
    F: Fn(&Path, &Metadata) -> T,
    T: Ord,
{
    Extract { inner: f, marker: PhantomData }
}

/// Returns a [`Sorter`] that sorts entries based on the [`Ord`] implementation of the extracted value.
#[inline]
pub const fn try_extract<F, R, T>(f: F) -> TryExtract<F, R, T>
where
    F: Fn(&Path, &Metadata) -> R,
    R: Try<Output = T>,
    T: Ord,
{
    TryExtract { inner: f, marker: PhantomData }
}

/// Returns a [`Sorter`] that sorts entries based on the [`Ord`] implementation of the extracted value, accounting for
/// depth.
#[inline]
pub const fn depth_extract<F, T>(f: F) -> DepthExtract<F, T>
where
    F: Fn(&Path, &Metadata, usize) -> T,
    T: Ord,
{
    DepthExtract { inner: f, marker: PhantomData }
}

/// A value that can be used to sort entries within a visit call.
#[must_use = "sorters do nothing unless provided to a visit call"]
pub trait Sort: Sized {
    /// Returns the ordering that should be used to sort the given entries.
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering;

    /// Returns the ordering that should be used to sort the given entries, accounting for depth.
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), depth: usize) -> Ordering;

    /// Reverses the order of this [`Sorter`].
    #[inline]
    fn reverse(self) -> Reverse<Self> {
        Reverse(self)
    }

    /// Chains this [`Sorter`] with the given [`Sorter`] in sequence, applying the second if the first returns
    /// [`Ordering::Equal`].
    #[inline]
    fn then<U: Sort>(self, other: U) -> Then<Self, U> {
        Then(self, other)
    }
}

/// Sorts entries based on the given sort function.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct By<F>(F);

impl<F> Sort for By<F>
where
    F: Fn((&Path, &Metadata), (&Path, &Metadata)) -> Ordering,
{
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        (self.0)(lhs, rhs)
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), _: usize) -> Ordering {
        self.sort(lhs, rhs)
    }
}

/// Sorts entries based on the given sort function, accounting for depth.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DepthBy<F>(F);

impl<F> Sort for DepthBy<F>
where
    F: Fn((&Path, &Metadata), (&Path, &Metadata), usize) -> Ordering,
{
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        self.depth_sort(lhs, rhs, 0)
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), depth: usize) -> Ordering {
        (self.0)(lhs, rhs, depth)
    }
}

/// Sorts entries based on the [`Ord`] implementation of the extracted value.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Extract<F, T> {
    /// The inner extraction function.
    inner: F,
    /// Retains the type marker for `T`.
    marker: PhantomData<fn(&Path, &Metadata) -> T>,
}

impl<F, T> Sort for Extract<F, T>
where
    F: Fn(&Path, &Metadata) -> T,
    T: Ord,
{
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        (self.inner)(lhs.0, lhs.1).cmp(&(self.inner)(rhs.0, rhs.1))
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), _: usize) -> Ordering {
        self.sort(lhs, rhs)
    }
}

/// Sorts entries based on the [`Ord`] implementation of the extracted value.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TryExtract<F, R, T> {
    /// The inner extraction function.
    inner: F,
    /// Retains the type marker for `T`.
    #[expect(clippy::type_complexity, reason = "no reason to abstract this out")]
    marker: PhantomData<(fn(&Path, &Metadata) -> R, fn(R) -> T)>,
}

impl<F, R, T> Sort for TryExtract<F, R, T>
where
    F: Fn(&Path, &Metadata) -> R,
    R: Try<Output = T>,
    T: Ord,
{
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        (self.inner)(lhs.0, lhs.1)
            .branch()
            .continue_value()
            .and_then(|l| (self.inner)(rhs.0, rhs.1).branch().continue_value().map(|r| l.cmp(&r)))
            .unwrap_or(Ordering::Greater)
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), _: usize) -> Ordering {
        self.sort(lhs, rhs)
    }
}

/// Sorts entries based on the [`Ord`] implementation of the extracted value, accounting for depth.
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DepthExtract<F, T> {
    /// The inner extraction function.
    inner: F,
    /// Retains the type marker for `T`.
    marker: PhantomData<fn(&Path, &Metadata, usize) -> T>,
}

impl<F, T> Sort for DepthExtract<F, T>
where
    F: Fn(&Path, &Metadata, usize) -> T,
    T: Ord,
{
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        self.depth_sort(lhs, rhs, 0)
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), depth: usize) -> Ordering {
        (self.inner)(lhs.0, lhs.1, depth).cmp(&(self.inner)(rhs.0, rhs.1, depth))
    }
}

/// Reverses the order of the inner [`Sorter`].
#[repr(transparent)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reverse<T>(T);

impl<T: Sort> Sort for Reverse<T> {
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        self.0.sort(lhs, rhs).reverse()
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), depth: usize) -> Ordering {
        self.0.depth_sort(lhs, rhs, depth).reverse()
    }
}

/// Chains two [`Sorter`]s together in sequence, applying the second if the first returns [`Ordering::Equal`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Then<T, U>(T, U);

impl<T: Sort, U: Sort> Sort for Then<T, U> {
    #[inline]
    fn sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata)) -> Ordering {
        self.0.sort(lhs, rhs).then_with(|| self.1.sort(lhs, rhs))
    }

    #[inline]
    fn depth_sort<'p>(&self, lhs: (&'p Path, &'p Metadata), rhs: (&'p Path, &'p Metadata), depth: usize) -> Ordering {
        self.0.depth_sort(lhs, rhs, depth).then_with(|| self.1.depth_sort(lhs, rhs, depth))
    }
}
