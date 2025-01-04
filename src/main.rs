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

//! An implementation of the `ls` command-line application.

// Panic prevention
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#![cfg_attr(debug_assertions, warn(clippy::todo, clippy::unimplemented))]
#![cfg_attr(not(debug_assertions), deny(clippy::todo, clippy::unimplemented))]
// Safety checks
#![deny(unsafe_code, clippy::missing_safety_doc, clippy::undocumented_unsafe_blocks)]
// General lints
#![warn(clippy::cargo, clippy::nursery, clippy::pedantic, missing_docs)]

const fn main() {}
