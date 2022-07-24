// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::private::Sealed;
use crate::traits::NtListType;

/// Designates a list as an NT doubly-linked list (`LIST_ENTRY`).
pub enum NtList {}
impl NtListType for NtList {}
impl Sealed for NtList {}
pub use nt_list_macros::NtList;
