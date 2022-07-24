// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::private::Sealed;
use crate::traits::NtListType;

/// Designates a list as an NT singly linked list (`SINGLE_LIST_ENTRY` structure of the Windows NT API).
pub enum NtSingleList {}

/// Singly linked list type (`SINGLE_LIST_ENTRY` structure of the Windows NT API)
impl NtListType for NtSingleList {}
impl Sealed for NtSingleList {}

/// Designates an empty enum as a singly linked list.
///
/// Technically, this macro implements [`NtTypedList`] with type set to [`enum@NtSingleList`].
///
/// [`NtTypedList`]: crate::traits::NtTypedList
pub use nt_list_macros::NtSingleList;
