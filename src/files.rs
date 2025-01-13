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

use std::cell::OnceCell;
use std::fs::Metadata;
use std::io::Result;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use self::filter::Filter;
use self::sort::Sort;

pub mod filter;
pub mod sort;

/// An entry returned by a visit call.
#[derive(Clone, Debug)]
pub struct Entry<'e> {
    /// The entry's file path.
    pub path: &'e Path,
    /// The entry's metadata.
    pub data: Option<&'e Metadata>,
    /// The entry's index in the current depth.
    pub index: usize,
    /// The total number of entries in the current depth.
    pub total: usize,
    /// Caches whether this entry has children.
    has_children_cache: OnceCell<bool>,
}

impl<'e> Entry<'e> {
    /// Creates a new [`Entry`] using the given path and optional data.
    #[inline]
    #[must_use]
    pub const fn new(path: &'e Path, data: Option<&'e Metadata>, index: usize, total: usize) -> Self {
        Self { path, data, index, total, has_children_cache: OnceCell::new() }
    }

    /// Creates a new [`Entry`] using the given path and optional data.
    ///
    /// This entry will have an index of 0, a total count of 1.
    #[inline]
    #[must_use]
    pub const fn root(path: &'e Path, data: Option<&'e Metadata>) -> Self {
        Self { path, data, index: 0, total: 1, has_children_cache: OnceCell::new() }
    }

    /// Returns whether this is the first entry in the current depth.
    #[inline]
    #[must_use]
    pub const fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Returns whether this is the last entry in the current depth.
    #[inline]
    #[must_use]
    pub const fn is_last(&self) -> bool {
        self.total == (self.index + 1)
    }

    /// Returns the number of entries remaining after this entry.
    #[inline]
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.total - (self.index + 1)
    }

    /// Returns `true` if this entry represents a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.data.map_or_else(|| self.path.is_dir(), Metadata::is_dir)
    }

    /// Returns `true` if this entry represents a file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.data.map_or_else(|| self.path.is_file(), Metadata::is_file)
    }

    /// Returns `true` if this entry represents a symbolic link.
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.data.map_or_else(|| self.path.is_symlink(), Metadata::is_symlink)
    }

    /// Returns `true` if this entry has an executable flag set.
    #[inline]
    #[must_use]
    pub fn is_executable(&self) -> bool {
        use crate::section::mode::permissions::{EXECUTE, MASK, test};

        self.data.is_some_and(|v| test::<MASK, EXECUTE>(v.mode()))
    }

    /// Returns `true` if this entry is considered 'hidden' based off its file name.
    #[inline]
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self::is_hidden(self.path)
    }

    /// Returns `true` if this entry represents a directory and has one or more entries within it.
    #[must_use]
    pub fn has_children(&self) -> bool {
        *self.has_children_cache.get_or_init(|| {
            // This call can be expensive, so we cache the result.
            self.is_dir() && std::fs::read_dir(self.path).is_ok_and(|mut v| v.next().is_some())
        })
    }
}

/// Visits all children of the given entry using the given closure.
///
/// The closure takes two arguments; a reference to the parent entries, and the child entry itself.
///
/// # Errors
///
/// This function will return an error if the entry's children could not be accessed or the closure fails.
pub fn visit_entries<F, S, V>(entry: &Rc<Entry>, filter: &F, sort: &S, mut visit: V) -> Result<()>
where
    F: Filter,
    S: Sort,
    V: FnMut(&[&Rc<Entry>], Rc<Entry>) -> Result<()>,
{
    let mut collection = std::fs::read_dir(entry.path)?
        .map(|v| v.and_then(|v| v.metadata().map(|d| (v.path(), d))))
        .filter(|v| v.as_ref().map_or(true, |v| filter.filter(&v.0, &v.1)))
        .collect::<Result<Box<[(PathBuf, Metadata)]>>>()?;

    collection.sort_unstable_by(|lhs, rhs| sort.sort((&lhs.0, &lhs.1), (&rhs.0, &rhs.1)));

    let total = collection.len();

    collection.iter().enumerate().try_for_each(|(index, (path, data))| {
        let child = Entry::new(path, Some(data), index, total);

        visit(&[entry], Rc::new(child))
    })
}

/// Visits all children of the given entry using the given closure recursively.
///
/// The closure takes two arguments; a reference to the parent entries, and the child entry itself.
///
/// # Errors
///
/// This function will return an error if an entry's children could not be accessed or the closure fails.
pub fn visit_entries_recursive<F, S, V>(entry: &Rc<Entry>, filter: &F, sort: &S, visit: &mut V) -> Result<()>
where
    F: Filter,
    S: Sort,
    V: FnMut(&[&Rc<Entry>], Rc<Entry>) -> Result<()>,
{
    #[inline]
    fn inner<F, S, V>(entries: &[&Rc<Entry>], filter: &F, sort: &S, visit: &mut V) -> Result<()>
    where
        F: Filter,
        S: Sort,
        V: FnMut(&[&Rc<Entry>], Rc<Entry>) -> Result<()>,
    {
        let Some(entry) = entries.last() else { unreachable!() };

        self::visit_entries(entry, filter, sort, |_, entry| {
            visit(entries, Rc::clone(&entry))?;

            if entry.has_children() {
                let mut new_entries = Vec::with_capacity(entries.len() + 1);

                new_entries.extend_from_slice(entries);
                new_entries.push(&entry);

                inner(&new_entries, filter, sort, visit)?;
            }

            Ok(())
        })
    }

    inner(&[entry], filter, sort, visit)
}

/// Returns `true` if the given path is considered 'hidden'.
pub fn is_hidden<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    path.as_ref().file_name().and_then(|v| v.as_bytes().first()).copied().is_some_and(|v| v == b'.')
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
            (Some(root), Some(path)) if components.is_empty() && root == path => {}
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
