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

//! Implements sections related to entry owners.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Result, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;

use uzers::{AllGroups, AllUsers, Groups, Users, UsersSnapshot};

use super::Section;
use crate::files::Entry;
use crate::writev;

/// The byte used when the user is missing.
pub const CHAR_MISSING: u8 = b'-';
/// The byte used for padding.
pub const CHAR_PADDING: u8 = b' ';

thread_local! {
    /// Caches the users and groups on this system.
    #[expect(unsafe_code, reason = "unsafe is required to retrieve these values")]
    // Safety: We only ever call potentially problematic code on this line, so there is not possible conflict.
    static USERS: UsersSnapshot = unsafe { UsersSnapshot::new() };
}

/// A [`Section`] that writes an entry's owner username.
#[derive(Clone, Copy, Debug)]
pub struct UserSection;

impl UserSection {
    /// Returns the maximum length that all user sections in the given directory will take up.
    fn max_len(parent: &Path) -> usize {
        thread_local! {
            static CACHE: RefCell<HashMap<Box<Path>, usize>> = RefCell::new(HashMap::new());
            static MAX_USER_LEN: usize = {
                USERS.with(|v| v.get_all_users().map(|v| v.name().len()).max().unwrap_or_default())
            };
        }

        CACHE.with(|cache| {
            if let Some(len) = cache.borrow().get(parent).copied() {
                return len;
            }

            let len = std::fs::read_dir(parent).ok().and_then(|v| {
                v.map_while(|v| v.and_then(|v| v.metadata()).ok())
                    .map_while(|v| USERS.with(|u| u.get_user_by_uid(v.uid())))
                    .map(|v| v.name().len())
                    .max()
            });
            let len = len.unwrap_or_else(|| MAX_USER_LEN.with(|v| *v));

            cache.borrow_mut().insert(Box::from(parent), len);

            len
        })
    }
}

impl Section for UserSection {
    fn write_plain<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(user) = entry.data.and_then(|v| USERS.with(|u| u.get_user_by_uid(v.uid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(user.name().len())];

        writev!(f, [user.name().as_encoded_bytes(), &padding])
    }

    fn write_color<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(user) = entry.data.and_then(|v| USERS.with(|u| u.get_user_by_uid(v.uid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]] in BrightBlack);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(user.name().len())];

        writev!(f, [user.name().as_encoded_bytes(), &padding] in BrightGreen)
    }
}

/// A [`Section`] that writes an entry's owner username.
#[derive(Clone, Copy, Debug)]
pub struct GroupSection;

impl GroupSection {
    /// Returns the maximum length that all group sections in the given directory will take up.
    fn max_len(parent: &Path) -> usize {
        thread_local! {
            static CACHE: RefCell<HashMap<Box<Path>, usize>> = RefCell::new(HashMap::new());
            static MAX_GROUP_LEN: usize = {
                USERS.with(|v| v.get_all_groups().map(|v| v.name().len()).max().unwrap_or_default())
            };
        }

        CACHE.with(|cache| {
            if let Some(len) = cache.borrow().get(parent).copied() {
                return len;
            }

            let len = std::fs::read_dir(parent).ok().and_then(|v| {
                v.map_while(|v| v.and_then(|v| v.metadata()).ok())
                    .map_while(|v| USERS.with(|u| u.get_group_by_gid(v.gid())))
                    .map(|v| v.name().len())
                    .max()
            });
            let len = len.unwrap_or_else(|| MAX_GROUP_LEN.with(|v| *v));

            cache.borrow_mut().insert(Box::from(parent), len);

            len
        })
    }
}

impl Section for GroupSection {
    fn write_plain<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(group) = entry.data.and_then(|v| USERS.with(|u| u.get_group_by_gid(v.gid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(group.name().len())];

        writev!(f, [group.name().as_encoded_bytes(), &padding])
    }

    fn write_color<W: Write>(&self, f: &mut W, parents: &[&Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(group) = entry.data.and_then(|v| USERS.with(|u| u.get_group_by_gid(v.gid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]] in BrightBlack);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(group.name().len())];

        writev!(f, [group.name().as_encoded_bytes(), &padding] in BrightYellow)
    }
}
