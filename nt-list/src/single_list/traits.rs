// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::traits::NtListType;

/// Designates a list as an NT singly-linked list (`SINGLE_LIST_ENTRY`).
pub enum NtSingleList {}
impl NtListType for NtSingleList {}
pub use nt_list_macros::NtSingleList;
