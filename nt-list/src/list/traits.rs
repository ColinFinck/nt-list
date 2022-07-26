// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::private::Sealed;
use crate::traits::NtListType;

/// Designates a list as an NT doubly linked list (`LIST_ENTRY` structure of the Windows NT API).
///
/// You usually want to use `#[derive(NtList)]` to implement [`NtTypedList`] with type set to `NtList`.
///
/// [`NtTypedList`]: crate::traits::NtTypedList
pub enum NtList {}

/// Doubly linked list type (`LIST_ENTRY` structure of the Windows NT API)
impl NtListType for NtList {}
impl Sealed for NtList {}

/// Designates an empty enum as a doubly linked list.
///
/// Technically, this macro implements [`NtTypedList`] with type set to [`enum@NtList`].
///
/// [`NtTypedList`]: crate::traits::NtTypedList
pub use nt_list_macros::NtList;
