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

//! Defines utilities for mapping out file tree structures.

use std::cmp::Ordering;
use std::path::{Component, Path, PathBuf};

/// Provides commonly-used sort functions for usage in [`visit_directory`][0].
///
/// [0]: crate::files::visit_directory
pub mod filtering {
    use std::path::Path;

    /// Filter out entries by the given visibility enum.
    #[inline]
    pub const fn visible(show_hidden: bool) -> impl Copy + Fn(&Path) -> bool {
        move |path| show_hidden || path.file_name().is_none_or(|n| !n.to_string_lossy().starts_with('.'))
    }

    /// Inverts a filtering function.
    #[inline]
    pub const fn not<F>(a: F) -> impl Copy + Fn(&Path) -> bool
    where
        F: Copy + Fn(&Path) -> bool,
    {
        move |path| !a(path)
    }

    /// Combines two filtering functions with the given closure.
    #[inline]
    pub const fn mix<A, B, F>(a: A, b: B, f: F) -> impl Copy + Fn(&Path) -> bool
    where
        A: Copy + Fn(&Path) -> bool,
        B: Copy + Fn(&Path) -> bool,
        F: Copy + Fn(bool, bool) -> bool,
    {
        move |path| f(a(path), b(path))
    }

    /// Combines two filtering functions with a logical and.
    #[inline]
    pub const fn and<A, B>(a: A, b: B) -> impl Copy + Fn(&Path) -> bool
    where
        A: Copy + Fn(&Path) -> bool,
        B: Copy + Fn(&Path) -> bool,
    {
        self::mix(a, b, |a, b| a && b)
    }

    /// Combines two filtering functions with a logical or.
    #[inline]
    pub const fn or<A, B>(a: A, b: B) -> impl Copy + Fn(&Path) -> bool
    where
        A: Copy + Fn(&Path) -> bool,
        B: Copy + Fn(&Path) -> bool,
    {
        self::mix(a, b, |a, b| a || b)
    }

    /// Combines two filtering functions with a logical xor.
    #[inline]
    pub const fn xor<A, B>(a: A, b: B) -> impl Copy + Fn(&Path) -> bool
    where
        A: Copy + Fn(&Path) -> bool,
        B: Copy + Fn(&Path) -> bool,
    {
        self::mix(a, b, |a, b| a ^ b)
    }
}

/// Provides commonly-used sort functions for usage in [`visit_directory`][0].
///
/// [0]: crate::files::visit_directory
pub mod sorting {
    use std::cmp::Ordering;
    use std::ops::Try;
    use std::path::Path;

    /// A constant for a valid [`None`] type for convenience.
    pub const NONE: Option<fn(&Path, &Path) -> Ordering> = None;

    /// Sorts paths using their file names.
    #[inline]
    pub const fn name() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        |lhs: &Path, rhs: &Path| lhs.file_name().zip(rhs.file_name()).map_or(Ordering::Equal, |(lhs, rhs)| lhs.cmp(rhs))
    }

    /// Sorts paths using their creation dates.
    #[inline]
    pub const fn created() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        self::try_extract(|path| path.symlink_metadata().and_then(|v| v.created()))
    }

    /// Sorts paths using their modification dates.
    #[inline]
    pub const fn modified() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        self::try_extract(|path| path.symlink_metadata().and_then(|v| v.modified()))
    }

    /// Sorts paths by files.
    #[inline]
    pub const fn files() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        self::reverse(self::extract(Path::is_file))
    }

    /// Sorts paths by symbolic links.
    #[inline]
    pub const fn symlinks() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        self::reverse(self::extract(Path::is_symlink))
    }

    /// Sorts paths by directories.
    #[inline]
    pub const fn directories() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        self::reverse(self::extract(Path::is_dir))
    }

    /// Sorts paths by hidden files.
    #[inline]
    pub const fn hidden() -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering {
        self::reverse(self::try_extract(|path| path.file_name().map(|s| s.to_string_lossy().starts_with('.'))))
    }

    /// Sorts paths using the given value extraction function.
    #[inline]
    pub const fn extract<F, T>(f: F) -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering
    where
        F: Fn(&Path) -> T,
        T: Ord,
    {
        move |lhs: &Path, rhs: &Path| f(lhs).cmp(&f(rhs))
    }

    /// Sorts paths using the given fallible value extraction function.
    #[inline]
    pub const fn try_extract<F, R>(f: F) -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering
    where
        F: Fn(&Path) -> R,
        R: Try<Output: Ord>,
    {
        move |lhs, rhs| match f(lhs).branch().continue_value().zip(f(rhs).branch().continue_value()) {
            Some((lhs, rhs)) => lhs.cmp(&rhs),
            None => Ordering::Equal,
        }
    }

    /// Inverts a sorting function.
    #[inline]
    pub const fn reverse<F>(f: F) -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering
    where
        F: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
    {
        move |lhs, rhs| f(lhs, rhs).reverse()
    }

    /// Combines two sorting functions with the given closure.
    #[inline]
    pub const fn mix<A, B, F>(a: A, b: B, f: F) -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering
    where
        A: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
        B: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
        F: Fn(Ordering, Ordering) -> Ordering,
    {
        move |lhs, rhs| f(a(lhs, rhs), b(lhs, rhs))
    }

    /// Combines two sorting functions by chaining the second after the first.
    #[inline]
    pub const fn then<A, B>(a: A, b: B) -> impl for<'p> Fn(&'p Path, &'p Path) -> Ordering
    where
        A: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
        B: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
    {
        self::mix(a, b, Ordering::then)
    }
}

/// Iterates over the file system, calling the given function for each entry.
///
/// The given closure accepts two arguments; the file path and the estimated amount of remaining entries.
///
/// # Errors
///
/// This function will return an error if iteration fails for any reason.
pub fn visit_directory<P, S, F, V>(root: P, sort: Option<&S>, filter: F, mut visit: V) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
    F: for<'p> Fn(&'p Path) -> bool,
    V: for<'de> FnMut(&'de Path, usize) -> std::io::Result<()>,
{
    let iterator = std::fs::read_dir(&root)?
        .map(|result| result.map(|entry| entry.path().into_boxed_path()))
        .filter(|result| result.as_ref().map_or(true, |path| filter(path)));

    let total_entries = std::fs::read_dir(&root)?.count();

    if let Some(sort) = sort {
        let mut entries: Box<[std::io::Result<Box<Path>>]> = iterator.collect();

        entries.sort_by(|l, r| match (l, r) {
            (Ok(l), Ok(r)) => sort(l, r),
            (Err(_), Ok(_)) => Ordering::Less,
            (Ok(_), Err(_)) => Ordering::Greater,
            (Err(_), Err(_)) => Ordering::Equal,
        });

        Box::into_iter(entries).enumerate().try_for_each(|(index, result)| {
            let remaining = total_entries.saturating_sub(index);

            result.and_then(|path| visit(&path, remaining))
        })
    } else {
        iterator.enumerate().try_for_each(|(index, result)| {
            let remaining = total_entries.saturating_sub(index);

            result.and_then(|path| visit(&path, remaining))
        })
    }
}

/// Iterates over the file system recursively, calling the given function for each entry.
///
/// The given closure accepts three arguments; the file path, the estimated amount of remaining entries, and the current
/// depth from the starting path.
///
/// # Errors
///
/// This function will return an error if iteration fails for any reason.
pub fn visit_directory_tree<P, S, F, V>(root: P, sort: Option<&S>, filter: F, visit: V) -> std::io::Result<()>
where
    P: AsRef<Path>,
    S: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
    F: Copy + for<'p> Fn(&'p Path) -> bool,
    V: Copy + for<'de> FnMut(&'de Path, usize, usize) -> std::io::Result<()>,
{
    #[inline]
    fn inner<P, S, F, V>(root: P, sort: Option<&S>, filter: F, mut visit: V, depth: usize) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        S: for<'p> Fn(&'p Path, &'p Path) -> Ordering,
        F: Copy + for<'p> Fn(&'p Path) -> bool,
        V: Copy + for<'de> FnMut(&'de Path, usize, usize) -> std::io::Result<()>,
    {
        self::visit_directory(root, sort, filter, |path, remaining| {
            visit(path, remaining, depth)?;

            if path.is_dir() { inner(path, sort, filter, visit, depth.saturating_add(1)) } else { Ok(()) }
        })
    }

    inner(root, sort, filter, visit, 0)
}

/// Returns a new path that represents the relative path from `root` to `path`.
///
/// Implementation roughly taken from the [`pathdiff`][0] crate.
///
/// [0]: https://github.com/Manishearth/pathdiff/blob/master/src/lib.rs
pub fn relativize<R, P>(root: R, path: P) -> Option<PathBuf>
where
    R: AsRef<Path>,
    P: AsRef<Path>,
{
    let root = root.as_ref();
    let path = path.as_ref();

    match (root.is_absolute(), path.is_absolute()) {
        (true, false) => return None,
        (false, true) => return Some(path.to_path_buf()),
        _ => {}
    }

    let mut root_components = root.components();
    let mut path_components = path.components();

    let capacity = {
        let (root_min, root_max) = root_components.size_hint();
        let (path_min, path_max) = path_components.size_hint();

        root_max.unwrap_or(root_min).max(path_max.unwrap_or(path_min))
    };

    let mut components = Vec::with_capacity(capacity);

    loop {
        match (root_components.next(), path_components.next()) {
            (None, None) => break,
            (None, Some(path)) => {
                components.push(path);
                components.extend(path_components);

                break;
            }
            (_, None) => components.push(Component::ParentDir),
            (Some(root), Some(path)) if components.is_empty() && root == path => continue,
            (Some(Component::CurDir), Some(path)) => components.push(path),
            (Some(Component::ParentDir), Some(_)) => return None,
            (Some(_), Some(path)) => {
                components.push(Component::ParentDir);
                components.extend(root_components.map(|_| Component::ParentDir));

                components.push(path);
                components.extend(path_components);

                break;
            }
        }
    }

    Some(components.into_iter().collect())
}
