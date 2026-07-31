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

//! Defines utilities for mapping out file tree structures.

use std::cell::OnceCell;
use std::fs::Metadata;
use std::io::Result;
use std::num::NonZero;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use recomposition::filter::Filter;
use recomposition::sort::{ListSortExt, Sort};

/// The bit pattern for a socket filetype.
pub const FILETYPE_SOCKET: u32 = 0o140_000;
/// The bit pattern for a symbolic link filetype.
pub const FILETYPE_SYMBOLIC_LINK: u32 = 0o120_000;
/// The bit pattern for a file filetype.
pub const FILETYPE_FILE: u32 = 0o100_000;
/// The bit pattern for a block device filetype.
pub const FILETYPE_BLOCK_DEVICE: u32 = 0o060_000;
/// The bit pattern for a directory filetype.
pub const FILETYPE_DIRECTORY: u32 = 0o040_000;
/// The bit pattern for a character device filetype.
pub const FILETYPE_CHARACTER_DEVICE: u32 = 0o020_000;
/// The bit pattern for a FIFO pipe filetype.
pub const FILETYPE_FIFO_PIPE: u32 = 0o010_000;

/// The bits set for read permissions.
pub const PERMISSION_READ: u32 = 0o000_444;
/// The bits set for write permissions.
pub const PERMISSION_WRITE: u32 = 0o000_222;
/// The bits set for execute permissions.
pub const PERMISSION_EXECUTE: u32 = 0o000_111;
/// The bits set for set-user-ID permissions.
pub const PERMISSION_SET_UID: u32 = 0o004_000;
/// The bits set for set-group-ID permissions.
pub const PERMISSION_SET_GID: u32 = 0o002_000;
/// The bits set for sticky permissions.
pub const PERMISSION_STICKY: u32 = 0o001_000;

/// An entry returned by a visit call.
#[derive(Clone, Debug)]
pub struct Entry<'e, F>
where
    F: Filter<(Box<Path>, EntryMetadata)>,
{
    /// The entry's filepath.
    pub path: Box<Path>,
    /// The entry's metadata.
    pub data: Option<EntryMetadata>,
    /// The entry's index in the current depth.
    pub index: usize,
    /// The total number of entries in the current depth.
    pub total: usize,
    /// The filter used to resolve entries.
    pub filter: &'e F,
    /// Caches whether this entry has children.
    has_children_cache: OnceCell<bool>,
    /// Caches whether this entry can be traversed like a directory.
    can_traverse_cache: OnceCell<bool>,
}

impl<'e, F> Entry<'e, F>
where
    F: Filter<(Box<Path>, EntryMetadata)>,
{
    /// Creates a new [`Entry`] using the given path and optional data.
    #[inline]
    #[must_use]
    pub const fn new(path: Box<Path>, data: Option<EntryMetadata>, index: usize, total: usize, filter: &'e F) -> Self {
        Self {
            path,
            data,
            index,
            total,
            filter,
            has_children_cache: OnceCell::new(),
            can_traverse_cache: OnceCell::new(),
        }
    }

    /// Creates a new [`Entry`] using the given path and optional data.
    ///
    /// This entry will have an index of 0, a total count of 1.
    #[inline]
    #[must_use]
    pub const fn root(path: Box<Path>, data: Option<EntryMetadata>, filter: &'e F) -> Self {
        Self::new(path, data, 0, 1, filter)
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

    /// Returns `true` if this entry represents a directory.
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.data.as_ref().map_or_else(|| self.path.is_dir(), |data| data.filetype().is_directory())
    }

    /// Returns `true` if this entry represents a file.
    #[inline]
    pub fn is_file(&self) -> bool {
        self.data.as_ref().map_or_else(|| self.path.is_file(), |data| data.filetype().is_file())
    }

    /// Returns `true` if this entry represents a symbolic link.
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.data.as_ref().map_or_else(|| self.path.is_symlink(), |data| data.filetype().is_symbolic_link())
    }

    /// Returns `true` if this entry has an executable flag set.
    #[inline]
    #[must_use]
    pub fn is_executable(&self) -> bool {
        self.data.as_ref().is_some_and(|data| data.permissions().has(PERMISSION_EXECUTE))
    }

    /// Returns `true` if this entry is considered 'hidden' based off its filename.
    #[inline]
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self::is_hidden(&self.path)
    }

    /// Returns `true` if this entry can be traversed like a directory.
    #[must_use]
    pub fn can_traverse(&self) -> bool {
        *self.can_traverse_cache.get_or_init(|| {
            self.is_dir() || (self.is_symlink() && std::fs::metadata(&self.path).is_ok_and(|data| data.is_dir()))
        })
    }

    /// Returns `true` if this entry represents a directory and has one or more entries within it.
    #[must_use]
    pub fn has_children(&self) -> bool {
        *self.has_children_cache.get_or_init(|| {
            // This call can be very expensive and slow, so we cache the result.
            std::fs::read_dir(&self.path).is_ok_and(|mut iterator| {
                // Search for at least one child that matches the filter.
                iterator.any(|result| {
                    result.is_ok_and(|entry| {
                        entry.metadata().is_ok_and(|metadata| {
                            self.filter.test(&(entry.path().into_boxed_path(), EntryMetadata::new(&metadata)))
                        })
                    })
                })
            })
        })
    }
}

/// A lighter representation of an entry's filesystem metadata.
#[derive(Clone, Copy, Debug)]
pub struct EntryMetadata {
    /// The entry's mode.
    pub mode: u32,
    /// The entry's size in bytes.
    pub size: u64,
    /// The creation time in seconds since the UNIX epoch.
    pub ctime: i64,
    /// The last access time in seconds since the UNIX epoch.
    pub atime: i64,
    /// The last modification time in seconds since the UNIX epoch.
    pub mtime: i64,
    /// The entry's user ID.
    pub uid: u32,
    /// The entry's group ID.
    pub gid: u32,
}

impl EntryMetadata {
    /// Creates a new [`EntryMetadata`] from the given [`Metadata`] reference.
    #[must_use]
    pub fn new(metadata: &Metadata) -> Self {
        Self {
            mode: metadata.mode(),
            size: metadata.size(),
            ctime: metadata.ctime(),
            atime: metadata.atime(),
            mtime: metadata.mtime(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        }
    }

    /// Returns the filetype of this [`EntryMetadata`].
    #[must_use]
    pub const fn filetype(&self) -> Filetype {
        Filetype(self.mode)
    }

    /// Returns the permissions of this [`EntryMetadata`].
    #[must_use]
    pub const fn permissions(&self) -> Permissions {
        Permissions(self.mode)
    }
}

/// Provides getters for filetype values.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filetype(pub(super) u32);

impl Filetype {
    /// Returns the bit pattern that represents the mode's filetype.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0 & 0xF000
    }

    /// Returns whether the mode's filetype matches the expected value.
    #[inline]
    #[must_use]
    pub const fn has(self, kind: u32) -> bool {
        self.get() == kind
    }

    /// Returns whether this mode is a symbolic link.
    #[inline]
    #[must_use]
    pub const fn is_symbolic_link(self) -> bool {
        self.has(self::FILETYPE_SYMBOLIC_LINK)
    }

    /// Returns whether this mode is a file.
    #[inline]
    #[must_use]
    pub const fn is_file(self) -> bool {
        self.has(self::FILETYPE_FILE)
    }

    /// Returns whether this mode is a directory.
    #[inline]
    #[must_use]
    pub const fn is_directory(self) -> bool {
        self.has(self::FILETYPE_DIRECTORY)
    }
}

/// Provides getters for permission values.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Permissions(pub(super) u32);

impl Permissions {
    /// Returns the bit collection that contains the mode's permissions.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0 & 0o007_777
    }

    /// Returns the bit collection that contains the mode's extra permissions.
    #[inline]
    #[must_use]
    pub const fn get_extra(self) -> u32 {
        self.0 & 0o007_000
    }

    /// Returns the bit collection that contains the mode's owner permissions.
    #[inline]
    #[must_use]
    pub const fn get_owner(self) -> u32 {
        self.0 & 0o000_700
    }

    /// Returns the bit collection that contains the mode's group permissions.
    #[inline]
    #[must_use]
    pub const fn get_group(self) -> u32 {
        self.0 & 0o000_070
    }

    /// Returns the bit collection that contains the mode's other permissions.
    #[inline]
    #[must_use]
    pub const fn get_other(self) -> u32 {
        self.0 & 0o000_007
    }

    /// Returns whether the mode's permissions contain the expected value.
    #[inline]
    #[must_use]
    pub const fn has(self, kind: u32) -> bool {
        (self.get() & kind) != 0
    }

    /// Returns whether the mode's extra permissions contain the expected value.
    #[inline]
    #[must_use]
    pub const fn has_extra(self, kind: u32) -> bool {
        (self.get_extra() & kind) != 0
    }

    /// Returns whether the mode's owner permissions contain the expected value.
    #[inline]
    #[must_use]
    pub const fn has_owner(self, kind: u32) -> bool {
        (self.get_owner() & kind) != 0
    }

    /// Returns whether the mode's group permissions contain the expected value.
    #[inline]
    #[must_use]
    pub const fn has_group(self, kind: u32) -> bool {
        (self.get_group() & kind) != 0
    }

    /// Returns whether the mode's other permissions contain the expected value.
    #[inline]
    #[must_use]
    pub const fn has_other(self, kind: u32) -> bool {
        (self.get_other() & kind) != 0
    }
}

/// Visits all children of the given entry using the given closure.
///
/// The closure takes two arguments; a reference to the parent entries, and the child entry itself.
///
/// # Errors
///
/// This function will return an error if the entry's children could not be accessed or the closure fails.
pub fn visit_entries<F, S, V>(entry: &Entry<F>, filter: &F, sort: &S, mut visit: V) -> Result<()>
where
    F: Filter<(Box<Path>, EntryMetadata)>,
    S: Sort<(Box<Path>, EntryMetadata)>,
    V: FnMut(&[&Entry<F>], &Entry<F>) -> Result<()>,
{
    if !entry.can_traverse() {
        return visit(&[], entry);
    }

    let mut collection = std::fs::read_dir(&entry.path)?
        .map(|v| v.and_then(|v| v.metadata().map(|d| (v.path().into_boxed_path(), EntryMetadata::new(&d)))))
        .filter(|v| v.as_ref().map_or(true, |v| filter.test(v)))
        .collect::<Result<Box<[(Box<Path>, EntryMetadata)]>>>()?;

    collection.sort_unstable_with(sort);

    let total = collection.len();

    collection.into_iter().enumerate().try_for_each(|(index, (path, data))| {
        let child = Entry::new(path, Some(data), index, total, filter);

        visit(&[entry], &child)
    })
}

/// Visits all children of the given entry using the given closure recursively.
///
/// The closure takes two arguments; a reference to the parent entries, and the child entry itself.
///
/// # Errors
///
/// This function will return an error if an entry's children could not be accessed or the closure fails.
pub fn visit_entries_recursive<F, S, V>(
    entry: &Entry<F>,
    max_depth: Option<NonZero<usize>>,
    filter: &F,
    sort: &S,
    visit: &mut V,
) -> Result<()>
where
    F: Filter<(Box<Path>, EntryMetadata)>,
    S: Sort<(Box<Path>, EntryMetadata)>,
    V: FnMut(&[&Entry<F>], &Entry<F>) -> Result<()>,
{
    #[inline]
    fn inner<F, S, V>(entries: &[&Entry<F>], max_depth: usize, filter: &F, sort: &S, visit: &mut V) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
        S: Sort<(Box<Path>, EntryMetadata)>,
        V: FnMut(&[&Entry<F>], &Entry<F>) -> Result<()>,
    {
        if max_depth == 0 {
            return Ok(());
        }

        let Some(entry) = entries.last() else { unreachable!() };

        self::visit_entries(entry, filter, sort, |_, entry| {
            visit(entries, entry)?;

            if entry.has_children() {
                let mut new_entries = Vec::with_capacity(entries.len() + 1);

                new_entries.extend_from_slice(entries);
                new_entries.push(entry);

                inner(&new_entries, max_depth.saturating_sub(1), filter, sort, visit)?;
            }

            Ok(())
        })
    }

    inner(&[entry], max_depth.map_or(usize::MAX, NonZero::get), filter, sort, visit)
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
/// Implementation roughly taken from the [`pathdiff`] crate.
///
/// [`pathdiff`]: https://github.com/Manishearth/pathdiff/blob/master/src/lib.rs
pub fn relativize<R, P>(root: R, path: P) -> Option<PathBuf>
where
    R: AsRef<Path>,
    P: AsRef<Path>,
{
    let root = root.as_ref();
    let path = path.as_ref();

    if path.is_absolute() {
        return Some(path.to_path_buf());
    }

    match (root.is_absolute(), path.is_absolute()) {
        (true, false) => return None,
        (false, true) => return Some(path.to_path_buf()),
        _ => {}
    }

    let mut root_components = root.components();
    let mut path_components = path.components();
    let mut components = PathBuf::with_capacity(root.as_os_str().len().max(path.as_os_str().len()));

    loop {
        match (root_components.next(), path_components.next()) {
            (None, None) => break,
            (None, Some(path)) => {
                components.push(path);
                components.extend(path_components);

                break;
            }
            (_, None) => components.push(Component::ParentDir),
            (Some(root), Some(path)) if components.as_os_str().is_empty() && root == path => {}
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

    Some(components)
}
