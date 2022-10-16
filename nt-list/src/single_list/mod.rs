// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
//! A singly linked list compatible to [`SINGLE_LIST_ENTRY`] of the Windows NT API.
//!
//! To make this list type-safe, `nt-list` first asks you to declare an empty enum, which then serves
//! as the `L` type parameter to distinguish different lists.
//! A list element can be part of multiple linked lists by having multiple entry fields in the element
//! structure.
//! You need to declare an empty enum for every entry field of every element structure.
//!
//! The empty enum is designated as a singly linked list via:
//!
//! ```ignore
//! #[derive(NtSingleList)]
//! enum MyList {}
//! ```
//!
//! Next you define your element structure, adding an [`NtSingleListEntry`] field for each singly linked
//! list you want your element to be part of.
//! A single [`NtSingleListEntry`] field can be marked with `#[boxed]` to make that list own the elements
//! and handle their memory allocation and deallocation:
//!
//! ```ignore
//! #[derive(Default, NtListElement)]
//! #[repr(C)]
//! struct MyElement {
//!     #[boxed]
//!     entry: NtSingleListEntry<Self, MyList>,
//!     value: i32,
//! }
//! ```
//!
//! You can then manage that list using the safe [`NtBoxingSingleListHead`] interface:
//!
//! ```ignore
//! let mut list = NtBoxingSingleListHead::<MyElement, MyList>::new();
//!
//! list.push_back(MyElement {
//!     value: 42,
//!     ..Default::default()
//! });
//! assert!(!list.is_empty());
//! ```
//!
//! For non-boxed entries, you can only use the [`NtSingleListHead`] interface.
//! It requires elements to be allocated beforehand on a stable address and be valid as long as
//! the list is used.
//! Without owning the elements, the Rust compiler cannot guarantee the validity of them.
//! This is why almost all [`NtSingleListHead`] functions are `unsafe`.
//! Fortunately, [`NtSingleListHead`] is usually only necessary when an element is part of multiple lists.
//!
//! [`SINGLE_LIST_ENTRY`]: https://docs.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-single_list_entry

mod base;
#[cfg(feature = "alloc")]
mod boxing;
mod traits;

pub use base::*;
#[cfg(feature = "alloc")]
pub use boxing::*;
pub use traits::*;
