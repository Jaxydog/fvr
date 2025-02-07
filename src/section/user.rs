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
use std::ffi::OsStr;
use std::io::{Result, Write};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;

use super::Section;
use crate::files::Entry;
use crate::files::filter::Filter;
use crate::writev;

/// The byte used when the user is missing.
pub const CHAR_MISSING: u8 = b'-';
/// The byte used for padding.
pub const CHAR_PADDING: u8 = b' ';
/// The assumed maximum length of a username.
pub const MAX_LEN: usize = 32;

/// A [`Section`] that writes an entry's owner username.
#[derive(Clone, Copy, Debug)]
pub struct UserSection;

impl UserSection {
    /// Returns the user name associated with the given user identifier.
    fn name(uid: u32) -> Option<Rc<OsStr>> {
        thread_local! {
            static CACHE: RefCell<HashMap<u32, Option<Rc<OsStr>>>> = RefCell::new(HashMap::new());
        }

        CACHE.with(|v| {
            v.borrow_mut().entry(uid).or_insert_with(|| uzers::get_user_by_uid(uid).map(|v| v.name().into())).clone()
        })
    }

    /// Returns the maximum length that all user sections in the given directory will take up.
    fn max_len(parent: &Path) -> usize {
        thread_local! {
            static CACHE: RefCell<HashMap<Box<Path>, usize>> = RefCell::new(HashMap::new());
        }

        CACHE.with(|cache| {
            if let Some(len) = cache.borrow().get(parent).copied() {
                return len;
            }

            let len = std::fs::read_dir(parent)
                .ok()
                .and_then(|v| {
                    v.map_while(|v| v.and_then(|v| v.metadata()).ok())
                        .map_while(|v| Self::name(v.uid()).map(|v| v.len()))
                        .max()
                })
                .unwrap_or(MAX_LEN);

            cache.borrow_mut().insert(Box::from(parent), len);

            len
        })
    }
}

impl Section for UserSection {
    fn write_plain<W: Write, F: Filter>(
        &self,
        f: &mut W,
        parents: &[&Rc<Entry<F>>],
        entry: &Rc<Entry<F>>,
    ) -> Result<()> {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(user) = entry.data.and_then(|v| Self::name(v.uid())) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(user.len())];

        writev!(f, [user.as_encoded_bytes(), &padding])
    }

    fn write_color<W: Write, F: Filter>(
        &self,
        f: &mut W,
        parents: &[&Rc<Entry<F>>],
        entry: &Rc<Entry<F>>,
    ) -> Result<()> {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(user) = entry.data.and_then(|v| Self::name(v.uid())) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(user.len())];

        writev!(f, [user.as_encoded_bytes(), &padding] in BrightGreen)
    }
}

/// A [`Section`] that writes an entry's owner username.
#[derive(Clone, Copy, Debug)]
pub struct GroupSection;

impl GroupSection {
    /// Returns the group name associated with the given group identifier.
    fn name(gid: u32) -> Option<Rc<OsStr>> {
        thread_local! {
            static CACHE: RefCell<HashMap<u32, Option<Rc<OsStr>>>> = RefCell::new(HashMap::new());
        }

        CACHE.with(|v| {
            v.borrow_mut().entry(gid).or_insert_with(|| uzers::get_group_by_gid(gid).map(|v| v.name().into())).clone()
        })
    }

    /// Returns the maximum length that all group sections in the given directory will take up.
    fn max_len(parent: &Path) -> usize {
        thread_local! {
            static CACHE: RefCell<HashMap<Box<Path>, usize>> = RefCell::new(HashMap::new());
        }

        CACHE.with(|cache| {
            if let Some(len) = cache.borrow().get(parent).copied() {
                return len;
            }

            let len = std::fs::read_dir(parent)
                .ok()
                .and_then(|v| {
                    v.map_while(|v| v.and_then(|v| v.metadata()).ok())
                        .map_while(|v| Self::name(v.gid()).map(|v| v.len()))
                        .max()
                })
                .unwrap_or(MAX_LEN);

            cache.borrow_mut().insert(Box::from(parent), len);

            len
        })
    }
}

impl Section for GroupSection {
    fn write_plain<W, F>(&self, f: &mut W, parents: &[&Rc<Entry<F>>], entry: &Rc<Entry<F>>) -> Result<()>
    where
        W: Write,
        F: Filter,
    {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(group) = entry.data.and_then(|v| Self::name(v.gid())) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(group.len())];

        writev!(f, [group.as_encoded_bytes(), &padding])
    }

    fn write_color<W, F>(&self, f: &mut W, parents: &[&Rc<Entry<F>>], entry: &Rc<Entry<F>>) -> Result<()>
    where
        W: Write,
        F: Filter,
    {
        let length = Self::max_len(parents[parents.len() - 1].path);

        let Some(group) = entry.data.and_then(|v| Self::name(v.gid())) else {
            return writev!(f, [&[CHAR_MISSING], &vec![b' '; length - 1]]);
        };

        let padding = vec![CHAR_PADDING; length.saturating_sub(group.len())];

        writev!(f, [group.as_encoded_bytes(), &padding] in BrightYellow)
    }
}
