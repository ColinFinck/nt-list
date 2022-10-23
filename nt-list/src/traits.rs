// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::private::Sealed;

/// The type (singly or doubly linked list) of an empty enum that implements [`NtTypedList`].
pub trait NtListType: Sealed {}

/// Designates an empty enum as an NT list of a specific type (singly or doubly linked list).
///
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
/// ```
/// # use nt_list::list::NtList;
///
/// #[derive(NtList)]
/// enum MyList {}
/// ```
///
/// [`NtList`]: enum@crate::list::NtList
/// [`NtListEntry`]: crate::list::NtListEntry
/// [`NtSingleList`]: enum@crate::single_list::NtSingleList
pub trait NtTypedList {
    /// Identifier of the list
    type T: NtListType;
}

/// Designates a structure as a list element with an entry field (e.g. [`NtListEntry`]) of a
/// particular NT list.
/// The entry field's position inside the list is given by implementing the `offset` method.
/// The NT list is identified via the enum that implements [`NtTypedList`].
///
/// You can implement this trait multiple times for a structure if it is part of multiple
/// lists (and therefore contains multiple entry fields).
///
/// The easiest way to implement this trait for all entry fields of a structure is to use
/// `derive` on the structure:
///
/// ```
/// # use nt_list::NtListElement;
/// # use nt_list::list::{NtList, NtListEntry};
///
/// # #[derive(NtList)]
/// # enum MyList {}
///
/// #[derive(NtListElement)]
/// #[repr(C)]
/// struct MyElement {
///     entry: NtListEntry<Self, MyList>,
///     value: i32,
/// }
/// ```
///
/// # Safety
///
/// This trait is unsafe, because the compiler cannot verify that the `offset` method has been
/// implemented correctly.
/// Safe functions rely on the offset pointing to an actual [`NtListEntry`] or [`NtSingleListEntry`].
/// This trait must also only be implemented for structures marked with `#[repr(C)]`.
///
/// It is therefore recommended to only derive this trait as described above and never implement
/// it manually.
///
/// [`NtListEntry`]: crate::list::NtListEntry
/// [`NtSingleListEntry`]: crate::single_list::NtSingleListEntry
pub unsafe trait NtListElement<L: NtTypedList> {
    /// Returns the byte offset to the entry field relative to the beginning of the
    /// element structure.
    fn offset() -> usize;
}

/// Implements the [`NtListElement`] and (optionally) [`NtBoxedListElement`] traits for the given
/// element structure.
///
/// Technically, this macro traverses the structure and looks for [`NtListEntry`] and [`NtSingleListEntry`]
/// fields.
/// For each entry, it takes its list type parameter `L` and implements [`NtListElement`] along with
/// the `offset` trait function for it.
///
/// If an entry is marked with the `#[boxed]` attribute, [`NtBoxedListElement`] is also implemented for
/// the structure.
///
/// [`NtListEntry`]: crate::list::NtListEntry
/// [`NtSingleListEntry`]: crate::single_list::NtSingleListEntry
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
/// ```
/// # use nt_list::NtListElement;
/// # use nt_list::list::{NtList, NtListEntry};
///
/// # #[derive(NtList)]
/// # enum MyList {}
///
/// #[derive(NtListElement)]
/// #[repr(C)]
/// struct MyElement {
///     #[boxed]
///     entry: NtListEntry<Self, MyList>,
///     value: i32,
/// }
/// ```
///
/// [`NtBoxingListHead`]: crate::list::NtBoxingListHead
/// [`NtListEntry`]: crate::list::NtListEntry
pub trait NtBoxedListElement {
    /// Identifier of the list
    type L: NtTypedList;
}
