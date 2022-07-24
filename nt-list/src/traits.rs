// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

pub trait NtListType {}

/// Designates an empty enum as an NT list of a specific type (singly/doubly-linked list).
/// You are supposed to define an empty enum and implement this trait for every list entry field
/// of every list element type in your program.
///
/// This is required, because a single element may be part of multiple NT lists, and henceforth
/// its element structure then contains multiple entry fields (e.g. [`NtListEntry`]).
/// To make all list functions insert and remove elements via the correct entry fields,
/// lists need to be uniquely identified, and this is what the empty enum types are for.
///
/// The easiest way to implement this trait is to use `derive` with the appropriate list type
/// ([`NtList`] or [`NtSingleList`]):
///
/// ```ignore
/// #[derive(NtList)]
/// enum MyList {}
/// ```
///
/// [`NtListEntry`]: super::base::NtListEntry
pub trait NtListOfType {
    type T: NtListType;
}

/// Designates a structure as a list element with an entry field (e.g. [`NtListEntry`]) of a
/// particular NT list (identified via the enum that implements [`NtListOfType`]).
///
/// You can implement this trait multiple times for a structure if it is part of multiple
/// lists (and therefore contains multiple entry fields).
///
/// The easiest way to implement this trait for all entry fields of a structure is to use
/// `derive` on the structure:
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
pub trait NtListElement<L: NtListOfType> {
    /// Returns the byte offset to the entry field relative to the beginning of the
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
/// entry field and use `derive` on the structure:
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
    type L: NtListOfType;
}
