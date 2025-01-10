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

use std::io::{Result, Write};
use std::os::unix::fs::MetadataExt;
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

    /// Caches the longest username length on the system.
    static MAX_USER_LEN: usize = {
        USERS.with(|v| v.get_all_users().map(|v| v.name().len()).max().unwrap_or_default())
    };

    /// Caches the longest group name length on the system.
    static MAX_GROUP_LEN: usize = {
        USERS.with(|v| v.get_all_groups().map(|v| v.name().len()).max().unwrap_or_default())
    };
}

/// A [`Section`] that writes an entry's owner username.
#[derive(Clone, Copy, Debug)]
pub struct UserSection;

impl Section for UserSection {
    fn write_plain<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = MAX_USER_LEN.with(|v| *v);
        let Some(user) = entry.data.and_then(|u| USERS.with(|v| v.get_user_by_uid(u.uid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(user.name().len())];

        writev!(f, [user.name().as_encoded_bytes(), &padding])
    }

    fn write_color<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = MAX_USER_LEN.with(|v| *v);
        let Some(user) = entry.data.and_then(|u| USERS.with(|v| v.get_user_by_uid(u.uid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]] in BrightBlack);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(user.name().len())];

        writev!(f, [user.name().as_encoded_bytes(), &padding] in BrightGreen)
    }
}

/// A [`Section`] that writes an entry's owner username.
#[derive(Clone, Copy, Debug)]
pub struct GroupSection;

impl Section for GroupSection {
    fn write_plain<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = MAX_GROUP_LEN.with(|v| *v);
        let Some(group) = entry.data.and_then(|u| USERS.with(|v| v.get_group_by_gid(u.gid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(group.name().len())];

        writev!(f, [group.name().as_encoded_bytes(), &padding])
    }

    fn write_color<W: Write>(&self, f: &mut W, _: &[Rc<Entry>], entry: &Rc<Entry>) -> Result<()> {
        let length = MAX_GROUP_LEN.with(|v| *v);
        let Some(group) = entry.data.and_then(|u| USERS.with(|v| v.get_group_by_gid(u.gid()))) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]] in BrightBlack);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(group.name().len())];

        writev!(f, [group.name().as_encoded_bytes(), &padding] in BrightYellow)
    }
}
