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

//! Implements a section that can display the owner of an entry.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::Metadata;
use std::io::{Result, StdoutLock};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;

use recomposition::filter::Filter;

use crate::files::{Entry, EntryMetadata};
use crate::section::Section;
use crate::writev;

/// The byte used when the user is missing.
const MISSING_CHARACTER: u8 = b'-';
/// The byte used for padding.
const PADDING_CHARACTER: u8 = b' ';

/// The maximum length of an identifier number.
const IDENTIFIER_MAXIMUM_LENGTH: usize = u32::MAX.ilog10() as usize + 1;
/// The padding used to fill remaining space after writing an identifier.
const IDENTIFIER_PADDING: &[u8] = &[PADDING_CHARACTER; IDENTIFIER_MAXIMUM_LENGTH];

/// The assumed maximum length of a name.
const NAME_MAXIMUM_LENGTH: usize = 32;
/// The padding used to fill remaining space after writing an identifier.
const NAME_PADDING: &[u8] = &[PADDING_CHARACTER; NAME_MAXIMUM_LENGTH];

thread_local! {
    /// Retains a map of user IDs to owner names.
    static USER_CACHE: RefCell<BTreeMap<u32, Option<Rc<OsStr>>>> = RefCell::new(BTreeMap::default());
    /// Retains a map of directories and the maximum length of all usernames within them.
    static USER_LENGTH_CACHE: RefCell<BTreeMap<Box<OsStr>, Option<usize>>> = RefCell::new(BTreeMap::default());

    /// Retains a map of group IDs to group names.
    static GROUP_CACHE: RefCell<BTreeMap<u32, Option<Rc<OsStr>>>> = RefCell::new(BTreeMap::default());
    /// Retains a map of directories and the maximum length of all group names within them.
    static GROUP_LENGTH_CACHE: RefCell<BTreeMap<Box<OsStr>, Option<usize>>> = RefCell::new(BTreeMap::default());
}

/// Determines what type of owner is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The owner user.
    User,
    /// The owner group.
    Group,
}

/// Determines how the owner is displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Display the owner's identifier.
    Identifier,
    /// Display the owner's name.
    Name,
}

/// A [`Section`] that writes an entry's owner.
#[derive(Clone, Copy, Debug)]
pub struct OwnerSection {
    /// The owner section type.
    pub kind: Kind,
    /// The owner section format.
    pub format: Format,
}

impl OwnerSection {
    /// Returns the name associated with the given identifier.
    fn resolve_name(self, id: u32) -> Option<Rc<OsStr>> {
        #[inline]
        fn get_user(uid: u32) -> Option<Rc<OsStr>> {
            uzers::get_user_by_uid(uid).map(|user| user.name().into())
        }
        #[inline]
        fn get_group(gid: u32) -> Option<Rc<OsStr>> {
            uzers::get_group_by_gid(gid).map(|group| group.name().into())
        }

        #[expect(clippy::type_complexity, reason = "this is a fairly trivial function pointer type")]
        let (name_cache, get): (_, fn(u32) -> Option<Rc<OsStr>>) = if matches!(self.kind, Kind::User) {
            (&USER_CACHE, get_user) //
        } else {
            (&GROUP_CACHE, get_group)
        };

        name_cache.with_borrow_mut(|cache| cache.entry(id).or_insert_with(|| get(id)).to_owned())
    }

    /// Returns the maximum length of an owner name in the given directory.
    fn maximum_name_length(self, path: &Path) -> Option<usize> {
        #[inline]
        fn get_uid(metadata: &Metadata) -> u32 {
            metadata.uid()
        }
        #[inline]
        fn get_gid(metadata: &Metadata) -> u32 {
            metadata.gid()
        }

        let (name_length_cache, get): (_, fn(&Metadata) -> u32) = if matches!(self.kind, Kind::User) {
            (&USER_LENGTH_CACHE, get_uid) // 
        } else {
            (&GROUP_LENGTH_CACHE, get_gid)
        };

        if let Some(length) = name_length_cache.with_borrow(|cache| cache.get(path.as_os_str()).copied()) {
            return length;
        }

        let length = std::fs::read_dir(path).ok().and_then(|iterator| {
            iterator
                .map_while(|result| result.ok()?.metadata().ok())
                .map_while(|metadata| self.resolve_name(get(&metadata)))
                .map(|name| name.len())
                .max()
        });

        name_length_cache.with_borrow_mut(|cache| cache.insert(path.as_os_str().into(), length));

        length
    }
}

impl Section for OwnerSection {
    fn write<F>(&self, color: bool, f: &mut StdoutLock<'_>, parents: &[&Entry<F>], entry: &Entry<F>) -> Result<()>
    where
        F: Filter<(Box<Path>, EntryMetadata)>,
    {
        let Some((id, name)) = entry.data.as_ref().and_then(|data| {
            let id = if matches!(self.kind, Kind::User) { data.uid } else { data.gid };

            Some((id, if matches!(self.format, Format::Name) { Some(self.resolve_name(id)?) } else { None }))
        }) else {
            let padding = if matches!(self.format, Format::Identifier) {
                Cow::Borrowed(&IDENTIFIER_PADDING[.. IDENTIFIER_MAXIMUM_LENGTH - 1])
            } else {
                let parent_path = parents.last().map_or_else(|| entry.path.parent(), |parent| Some(&parent.path));
                let length = parent_path.and_then(|path| self.maximum_name_length(path)).unwrap_or(NAME_MAXIMUM_LENGTH);

                if length < NAME_MAXIMUM_LENGTH {
                    Cow::Borrowed(&NAME_PADDING[.. length - 1])
                } else {
                    Cow::Owned(vec![PADDING_CHARACTER; length - 1])
                }
            };

            return if color {
                writev!(f, [&[MISSING_CHARACTER], &padding] in BrightBlack)
            } else {
                writev!(f, [&[MISSING_CHARACTER], &padding])
            };
        };

        if matches!(self.format, Format::Identifier) {
            let mut buffer = itoa::Buffer::new();
            let bytes = buffer.format(id).as_bytes();

            let padding = &IDENTIFIER_PADDING[.. IDENTIFIER_MAXIMUM_LENGTH - bytes.len()];

            if !color {
                writev!(f, [bytes, padding])
            } else if matches!(self.kind, Kind::User) {
                writev!(f, [bytes, padding] in BrightGreen)
            } else {
                writev!(f, [bytes, padding] in BrightYellow)
            }
        } else {
            let Some(name) = name else { unreachable!() };
            let bytes = name.as_encoded_bytes();

            let parent_path = parents.last().map_or_else(|| entry.path.parent(), |parent| Some(&parent.path));
            let length = parent_path.and_then(|path| self.maximum_name_length(path)).unwrap_or(NAME_MAXIMUM_LENGTH);

            let padding = if length < NAME_MAXIMUM_LENGTH {
                Cow::Borrowed(&NAME_PADDING[.. length - bytes.len()])
            } else {
                Cow::Owned(vec![PADDING_CHARACTER; length - bytes.len()])
            };

            if !color {
                writev!(f, [bytes, &padding])
            } else if matches!(self.kind, Kind::User) {
                writev!(f, [bytes, &padding] in BrightGreen)
            } else {
                writev!(f, [bytes, &padding] in BrightYellow)
            }
        }
    }
}
