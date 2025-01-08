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

//! Implements entry owner user and group displays.

use std::io::{Result, StdoutLock};
use std::os::unix::fs::MetadataExt;

use uzers::{AllGroups, AllUsers, Groups, Users, UsersSnapshot};

use super::{Show, ShowData};
use crate::arguments::model::Arguments;
use crate::{optionally_vector, optionally_vector_color};

thread_local! {
    /// Caches the users and groups on this system.
    #[expect(unsafe_code, reason = "unsafe is required to retrieve these values")]
    // Safety: We only ever call potentially problematic code on this line, so there is not possible conflict.
    static USERS: UsersSnapshot = unsafe { UsersSnapshot::new() };

    /// Caches the maximum username length on the system.
    static MAX_USER_LEN: usize = {
        USERS.with(|v| v.get_all_users().map(|v| v.name().len()).max().unwrap_or_default())
    };

    /// Caches the maximum group name length on the system.
    static MAX_GROUP_LEN: usize = {
        USERS.with(|v| v.get_all_groups().map(|v| v.name().len()).max().unwrap_or_default())
    };
}

/// Renders a file entry's owner user.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct User;

impl Show for User {
    fn show_plain(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let max_length = MAX_USER_LEN.with(|v| *v);
        let Some(user) = entry.data.and_then(|u| USERS.with(|v| v.get_user_by_uid(u.uid()))) else {
            return optionally_vector!(f, [&vec![b' '; max_length]]);
        };
        let padding = vec![b' '; max_length.saturating_sub(user.name().len())];

        optionally_vector!(f, [user.name().as_encoded_bytes(), &padding])
    }

    fn show_color(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let max_length = MAX_USER_LEN.with(|v| *v);
        let Some(user) = entry.data.and_then(|u| USERS.with(|v| v.get_user_by_uid(u.uid()))) else {
            return optionally_vector_color!(f, BrightBlack, [b"-", &vec![b' '; max_length - 1]]);
        };
        let padding = vec![b' '; max_length.saturating_sub(user.name().len())];

        optionally_vector_color!(f, BrightGreen, [user.name().as_encoded_bytes(), &padding])
    }
}

/// Renders a file entry's owner group.
#[must_use = "render implementations do nothing unless used"]
#[derive(Clone, Copy, Debug)]
pub struct Group;

impl Show for Group {
    fn show_plain(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let max_length = MAX_GROUP_LEN.with(|v| *v);
        let Some(group) = entry.data.and_then(|u| USERS.with(|v| v.get_group_by_gid(u.gid()))) else {
            return optionally_vector!(f, [&vec![b' '; max_length]]);
        };
        let padding = vec![b' '; max_length.saturating_sub(group.name().len())];

        optionally_vector!(f, [group.name().as_encoded_bytes(), &padding])
    }

    fn show_color(&self, _: &Arguments, f: &mut StdoutLock, entry: ShowData<'_>) -> Result<()> {
        let max_length = MAX_GROUP_LEN.with(|v| *v);
        let Some(group) = entry.data.and_then(|u| USERS.with(|v| v.get_group_by_gid(u.gid()))) else {
            return optionally_vector_color!(f, BrightBlack, [b"-", &vec![b' '; max_length - 1]]);
        };
        let padding = vec![b' '; max_length.saturating_sub(group.name().len())];

        optionally_vector_color!(f, BrightYellow, [group.name().as_encoded_bytes(), &padding])
    }
}
