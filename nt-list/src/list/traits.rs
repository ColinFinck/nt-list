// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Designates an empty enum as an NT doubly-linked list.
/// You are supposed to define an empty enum and implement this trait for every list entry field
/// of every list element type in your program.
///
/// This is required, because a single element may be part of multiple NT doubly-linked lists, and
/// henceforth its element structure then contains multiple [`NtListEntry`] fields.
/// To make all list functions insert and remove elements via the correct [`NtListEntry`] fields,
/// lists need to be uniquely identified, and this is what the empty enum types are for.
///
/// The easiest way to implement this trait is to use `derive`:
///
/// ```ignore
/// #[derive(NtList)]
/// enum MyList {}
/// ```
///
/// [`NtListEntry`]: super::base::NtListEntry
pub trait NtList {}
pub use nt_list_macros::NtList;

/// Designates a structure as a list element with a [`NtListEntry`] field of a particular
/// NT doubly-linked list (identified via an empty enum that implements [`NtList`]).
///
/// You can implement this trait multiple times for a structure if it is part of multiple
/// lists (and therefore contains multiple [`NtListEntry`] fields).
///
/// The easiest way to implement this trait for all [`NtListEntry`] fields of a structure
/// is to use `derive` on the structure:
///
/// ```ignore
/// #[derive(NtListElement)]
/// #[repr(C)]
/// struct MyElement {
///     entry: NtListEntry<Self, MyList>,
///     value: i32,
/// }
/// ```
///
/// [`NtListEntry`]: super::base::NtListEntry
pub trait NtListElement<L: NtList> {
    /// Returns the byte offset to the [`NtListEntry`] field relative to the beginning of the
    /// element structure.
    ///
    /// [`NtListEntry`]: super::base::NtListEntry
    fn offset() -> usize;
}
pub use nt_list_macros::NtListElement;

/// Enables [`NtBoxingListHead`] for a list element structure.
///
/// While an element may be part of multiple lists, only one list may have ownership of the element
/// and handle its memory allocation and deallocation.
/// Therefore, `NtBoxedListElement` can only be implemented once per list element structure.
///
/// The easiest way to implement this trait is to use the `#[boxed]` attribute for the appropriate
/// [`NtListEntry`] field and use `derive` on the structure:
///
/// ```ignore
/// #[derive(NtListElement)]
/// #[repr(C)]
/// struct MyElement {
///     #[boxed]
///     entry: NtListEntry<Self, MyList>,
///     value: i32,
/// }
/// ```
///
/// [`NtBoxingListHead`]: super::boxing::NtBoxingListHead
/// [`NtListEntry`]: super::base::NtListEntry
pub trait NtBoxedListElement {
    type L: NtList;
}
