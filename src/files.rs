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

use std::fs::Metadata;
use std::io::Result;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use self::filter::Filter;
use self::sort::Sort;
use crate::display::mode::permissions::EXECUTE;

pub mod filter;
pub mod sort;

/// Iterates over the file system, calling the given function for each entry.
///
/// The given closure accepts three arguments; the file path, its metadata, and the estimated amount of remaining
/// entries.
///
/// # Errors
///
/// This function will return an error if iteration fails for any reason.
pub fn visit<P, S, F, V>(path: P, filter: &F, sort: &S, mut visit: V) -> Result<()>
where
    P: AsRef<Path>,
    F: Filter,
    S: Sort,
    V: FnMut(&Path, &Metadata, usize) -> Result<()>,
{
    let mut collection = std::fs::read_dir(path.as_ref())?
        .map(|v| v.and_then(|v| Ok((v.path(), v.metadata()?))))
        .filter(|v| v.as_ref().is_ok_and(|(p, d)| filter.filter(p, d)))
        .collect::<Result<Box<[(PathBuf, Metadata)]>>>()?;

    collection.sort_unstable_by(|(l_path, l_data), (r_path, r_data)| sort.sort((l_path, l_data), (r_path, r_data)));

    collection.iter().enumerate().try_for_each(|(index, (path, data))| visit(path, data, collection.len() - index))
}

/// Iterates over the file system recursively, calling the given function for each entry.
///
/// The given closure accepts four arguments; the file path, its metadata, the estimated amount of remaining entries,
/// and the current depth from the starting path.
///
/// # Errors
///
/// This function will return an error if iteration fails for any reason.
pub fn visit_recursive<P, S, F, V>(path: P, filter: &F, sort: &S, visit: V) -> Result<()>
where
    P: AsRef<Path>,
    F: Filter,
    S: Sort,
    V: FnMut(&Path, &Metadata, usize, usize) -> Result<()>,
{
    #[inline]
    fn recurse<P, S, F, V>(path: P, filter: &F, sort: &S, mut visit: V, depth: usize) -> Result<()>
    where
        P: AsRef<Path>,
        F: Filter,
        S: Sort,
        V: FnMut(&Path, &Metadata, usize, usize) -> Result<()>,
    {
        self::visit(
            &path,
            &self::filter::by(|path, data| filter.depth_filter(path, data, depth)),
            &self::sort::by(|lhs, rhs| sort.depth_sort(lhs, rhs, depth)),
            |path, data, remaining| visit(path, data, remaining, depth),
        )?;

        recurse(path, filter, sort, visit, depth.saturating_add(1))
    }

    recurse(path, filter, sort, visit, 0)
}

/// Returns `true` if the given path is considered 'hidden'.
pub fn is_hidden<P>(path: P) -> bool
where
    P: AsRef<Path>,
{
    path.as_ref().file_name().and_then(|v| v.as_bytes().first()).copied().is_some_and(|v| v == b'.')
}

/// Returns `true` if the given metadata is executable.
#[must_use]
pub fn is_executable(data: &Metadata) -> bool {
    data.mode() & EXECUTE != 0
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
