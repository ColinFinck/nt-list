// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
//! A doubly linked list compatible to [`LIST_ENTRY`] of the Windows NT API.
//!
//! To make this list type-safe, `nt-list` first asks you to declare an empty enum, which then serves
//! as the `L` type parameter to distinguish different lists.
//! A list element can be part of multiple linked lists by having multiple entry fields in the element
//! structure.
//! You need to declare an empty enum for every entry field of every element structure.
//!
//! The empty enum is designated as a doubly linked list via:
//!
//! ```ignore
//! #[derive(NtList)]
//! enum MyList {}
//! ```
//!
//! Next you define your element structure, adding an [`NtListEntry`] field for each doubly linked
//! list you want your element to be part of.
//! A single [`NtListEntry`] field can be marked with `#[boxed]` to make that list own the elements
//! and handle their memory allocation and deallocation:
//!
//! ```ignore
//! #[derive(Default, NtListElement)]
//! #[repr(C)]
//! struct MyElement {
//!     #[boxed]
//!     entry: NtListEntry<Self, MyList>,
//!     value: i32,
//! }
//! ```
//!
//! You can then manage that list using the safe [`NtBoxingListHead`] interface:
//!
//! ```ignore
//! moveit! {
//!     let mut list = NtBoxingListHead::<MyElement, MyList>::new();
//! }
//!
//! list.as_mut().push_back(MyElement {
//!     value: 42,
//!     ..Default::default()
//! });
//! assert!(!list.as_ref().is_empty());
//! ```
//!
//! The last link of a `LIST_ENTRY` doubly linked list points back to the list header.
//! This requires the address of the list header to be stable.
//! Therefore, the list address is pinned on creation by using the [`moveit`] crate, and
//! all doubly linked list functions require pinned references.
//!
//! For non-boxed entries, you can only use the [`NtListHead`] interface.
//! It requires elements to be allocated beforehand on a stable address and be valid as long as
//! the list is used.
//! Without owning the elements, the Rust compiler cannot guarantee the validity of them.
//! This is why almost all [`NtListHead`] functions are `unsafe`.
//! Fortunately, [`NtListHead`] is usually only necessary when an element is part of multiple lists.
//!
//! [`LIST_ENTRY`]: https://docs.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-list_entry
//! [`moveit`]: https://crates.io/crates/moveit

mod base;
#[cfg(feature = "alloc")]
mod boxing;
mod traits;

pub use base::*;
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub use boxing::*;
pub use traits::*;
