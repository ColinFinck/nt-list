// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Designates an empty enum type to identify an NT doubly-linked list.
/// You are supposed to define an empty enum type for every NT doubly-linked list in your program
/// and implement this trait.
///
/// This is required, because a single element may be part of multiple NT doubly-linked lists, and
/// henceforth its element structure then contains multiple [`ListEntry`] fields.
/// To make all list functions insert and remove elements via the correct [`ListEntry`] fields,
/// lists need to be uniquely identified, and this is what the empty enum types are for.
///
/// [`ListEntry`]: super::base::ListEntry
pub trait IsDoublyLinkedList {}

/// Designates a structure type as an element structure with a [`ListEntry`] field of a particular
/// NT doubly-linked list (identified via an empty enum type that implements [`IsDoublyLinkedList`]).
///
/// This trait can be implemented multiple times for the same element structure if the element
/// is part of multiple lists.
///
/// [`ListEntry`]: super::base::ListEntry
pub trait HasListEntry<L: IsDoublyLinkedList> {
    /// Returns the byte offset to the [`ListEntry`] field relative to the beginning of the
    /// element structure.
    ///
    /// [`ListEntry`]: super::base::ListEntry
    fn offset() -> usize;
}

/// Enables [`BoxingListHead`] for an element structure type.
///
/// While an element may simultaneously be part of multiple lists, only one of them may have
/// ownership of the elements and handle their memory allocation and deallocation.
/// This is achieved via the design of `BoxedListEntry`, which only allows to specify a single list
/// type per element structure type.
///
/// [`BoxingListHead`]: super::boxing::BoxingListHead
pub trait BoxedListEntry {
    type L: IsDoublyLinkedList;
}
