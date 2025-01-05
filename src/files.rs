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

use std::{cmp::Ordering, path::Path};

/// Determines the manner in which a directory is visited.
#[must_use]
#[derive(Clone, Debug)]
pub struct VisitSettings<P> {
    /// The root directory.
    root: P,
    /// Sorts file paths by the given ordering function.
    sorter: Option<fn(&Path, &Path) -> Ordering>,
    /// Filters out file paths that do not match the predicate.
    filter: Option<fn(&Path) -> bool>,
}

impl<P: AsRef<Path>> VisitSettings<P> {
    /// Creates a new [`VisitSettings<P>`].
    pub const fn new(root: P) -> Self {
        Self { root, sorter: None, filter: None }
    }

    /// Sorts visited file paths by the given ordering function.
    pub const fn sorted(mut self, f: fn(&Path, &Path) -> Ordering) -> Self {
        self.sorter = Some(f);

        self
    }

    /// Filters out entries that do not match the given predicate.
    pub const fn filter(mut self, f: fn(&Path) -> bool) -> Self {
        self.filter = Some(f);

        self
    }

    /// Returns a copy of this settings value with the given root.
    const fn copy_with_root<U: AsRef<Path>>(&self, root: U) -> VisitSettings<U> {
        VisitSettings { root, sorter: self.sorter, filter: self.filter }
    }
}

/// Iterates over the file system, calling the given function for each entry.
///
/// # Errors
///
/// This function will return an error if iteration fails for any reason.
pub fn visit_directory<P, F>(settings: &VisitSettings<P>, mut f: F) -> std::io::Result<()>
where
    P: AsRef<Path>,
    F: for<'de> FnMut(&'de Path) -> std::io::Result<()>,
{
    let VisitSettings { ref root, sorter, filter } = settings;

    let mut iterator = std::fs::read_dir(root)?
        .map(|result| result.map(|entry| entry.path().into_boxed_path()))
        .filter(|result| filter.is_none_or(|filter| result.as_ref().map_or(true, |path| filter(path))));

    if let Some(sorter) = sorter {
        let mut entries: Box<[std::io::Result<Box<Path>>]> = iterator.collect();

        entries.sort_by(|l, r| match (l, r) {
            (Ok(l), Ok(r)) => sorter(l, r),
            (Err(_), Ok(_)) => Ordering::Less,
            (Ok(_), Err(_)) => Ordering::Greater,
            (Err(_), Err(_)) => Ordering::Equal,
        });

        Box::into_iter(entries).try_for_each(|result| result.and_then(|path| f(&path)))
    } else {
        iterator.try_for_each(|result| result.and_then(|path| f(&path)))
    }
}

/// Iterates over the file system recursively, calling the given function for each entry.
///
/// # Errors
///
/// This function will return an error if iteration fails for any reason.
pub fn visit_directory_tree<P, F>(settings: &VisitSettings<P>, f: F) -> std::io::Result<()>
where
    P: AsRef<Path>,
    F: Copy + for<'de> FnMut(&'de Path, usize) -> std::io::Result<()>,
{
    fn inner<P, F>(settings: &VisitSettings<P>, mut f: F, depth: usize) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        F: Copy + for<'de> FnMut(&'de Path, usize) -> std::io::Result<()>,
    {
        self::visit_directory(settings, |path| {
            f(path, depth)?;

            if path.is_dir() {
                inner(&settings.copy_with_root(path), f, depth.saturating_add(1))?;
            }

            Ok(())
        })
    }

    inner(settings, f, 0)
}
