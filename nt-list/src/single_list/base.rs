// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::ptr;

use super::traits::NtSingleList;
use crate::traits::{NtListElement, NtTypedList};

/// A singly linked list header compatible to [`SINGLE_LIST_ENTRY`] of the Windows NT API.
///
/// This variant requires elements to be allocated beforehand on a stable address and be
/// valid as long as the list is used.
/// As the Rust compiler cannot guarantee the validity of them, almost all `NtSingleListHead`
/// functions are `unsafe`.
/// You almost always want to use [`NtBoxingSingleListHead`] over this.
///
/// See the [module-level documentation](crate::single_list) for more details.
///
/// This structure substitutes the `SINGLE_LIST_ENTRY` structure of the Windows NT API for the list header.
///
/// [`NtBoxingSingleListHead`]: crate::single_list::NtBoxingSingleListHead
/// [`SINGLE_LIST_ENTRY`]: https://docs.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-single_list_entry
#[repr(C)]
pub struct NtSingleListHead<E: NtListElement<L>, L: NtTypedList<T = NtSingleList>> {
    pub(crate) next: *mut NtSingleListEntry<E, L>,
}

impl<E, L> NtSingleListHead<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    /// Creates a new singly linked list.
    pub fn new() -> Self {
        Self {
            next: ptr::null_mut(),
        }
    }

    /// Removes all elements from the list.
    ///
    /// This operation computes in *O*(*1*) time, because it only resets the forward link of the header.
    pub fn clear(&mut self) {
        self.next = ptr::null_mut();
    }

    /// Returns the [`NtSingleListEntry`] for the given element.
    pub(crate) fn entry(element: &mut E) -> *mut NtSingleListEntry<E, L> {
        let element_address = element as *mut _ as usize;
        let entry_address = element_address + E::offset();
        entry_address as *mut NtSingleListEntry<E, L>
    }

    /// Provides a reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn front(&self) -> Option<&E> {
        (!self.is_empty()).then(|| (&*self.next).containing_record())
    }

    /// Provides a mutable reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn front_mut(&mut self) -> Option<&mut E> {
        (!self.is_empty()).then(|| (&mut *self.next).containing_record_mut())
    }

    /// Returns `true` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn is_empty(&self) -> bool {
        self.next.is_null()
    }

    /// Returns an iterator yielding references to each element of the list.
    pub unsafe fn iter(&self) -> Iter<E, L> {
        Iter {
            current: self.next,
            phantom: PhantomData,
        }
    }

    /// Returns an iterator yielding mutable references to each element of the list.
    pub unsafe fn iter_mut(&mut self) -> IterMut<E, L> {
        IterMut {
            current: self.next,
            phantom: PhantomData,
        }
    }

    /// Counts all elements and returns the length of the list.
    ///
    /// This operation computes in *O*(*n*) time.
    pub unsafe fn len(&self) -> usize {
        self.iter().count()
    }

    /// Removes the first element from the list and returns it, or `None` if the list is empty.
    ///
    /// This function substitutes [`PopEntryList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`PopEntryList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-popentrylist
    pub unsafe fn pop_front(&mut self) -> Option<&mut E> {
        (!self.is_empty()).then(|| {
            let entry = &mut *self.next;
            self.next = entry.next;
            entry.containing_record_mut()
        })
    }

    /// Appends an element to the front of the list.
    ///
    /// This function substitutes [`PushEntryList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`PushEntryList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-pushentrylist
    pub unsafe fn push_front(&mut self, element: &mut E) {
        let entry = Self::entry(element);

        (*entry).next = self.next;
        self.next = entry;
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// In other words, remove all elements `e` for which `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the original order,
    /// and preserves the order of the retained elements.
    ///
    /// This operation computes in *O*(*n*) time.
    pub unsafe fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        let mut previous = (self as *mut Self).cast();
        let mut current = self.next;

        while !current.is_null() {
            let element = (&mut *current).containing_record_mut();

            if f(element) {
                previous = current;
            } else {
                (*previous).next = (*current).next;
            }

            current = (*current).next;
        }
    }
}

impl<E, L> Default for NtSingleListHead<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over the elements of a singly linked list.
///
/// This iterator is returned from the [`NtSingleListHead::iter`] and
/// [`NtBoxingSingleListHead::iter`] functions.
///
/// [`NtBoxingSingleListHead::iter`]: crate::single_list::NtBoxingSingleListHead::iter
pub struct Iter<'a, E: NtListElement<L>, L: NtTypedList<T = NtSingleList>> {
    current: *const NtSingleListEntry<E, L>,
    phantom: PhantomData<&'a NtSingleListHead<E, L>>,
}

impl<'a, E, L> Iterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    type Item = &'a E;

    fn next(&mut self) -> Option<&'a E> {
        if self.current.is_null() {
            None
        } else {
            unsafe {
                let element = (&*self.current).containing_record();
                self.current = (*self.current).next;
                Some(element)
            }
        }
    }
}

impl<'a, E, L> FusedIterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
}

/// Mutable iterator over the elements of a singly linked list.
///
/// This iterator is returned from the [`NtSingleListHead::iter_mut`] and
/// [`NtBoxingSingleListHead::iter_mut`] functions.
///
/// [`NtBoxingSingleListHead::iter_mut`]: crate::single_list::NtBoxingSingleListHead::iter_mut
pub struct IterMut<'a, E: NtListElement<L>, L: NtTypedList<T = NtSingleList>> {
    current: *mut NtSingleListEntry<E, L>,
    phantom: PhantomData<&'a mut NtSingleListHead<E, L>>,
}

impl<'a, E, L> Iterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    type Item = &'a mut E;

    fn next(&mut self) -> Option<&'a mut E> {
        if self.current.is_null() {
            None
        } else {
            unsafe {
                let element = (&mut *self.current).containing_record_mut();
                self.current = (*self.current).next;
                Some(element)
            }
        }
    }
}

impl<'a, E, L> FusedIterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
}

/// This structure substitutes the `SINGLE_LIST_ENTRY` structure of the Windows NT API for actual list entries.
#[derive(Debug)]
#[repr(C)]
pub struct NtSingleListEntry<E: NtListElement<L>, L: NtTypedList<T = NtSingleList>> {
    pub(crate) next: *mut NtSingleListEntry<E, L>,
}

impl<E, L> NtSingleListEntry<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    /// Allows the creation of an `NtSingleListEntry`, but leaves all fields uninitialized.
    ///
    /// Its fields are only initialized when an entry is pushed to a list.
    #[allow(clippy::uninit_assumed_init)]
    pub fn new() -> Self {
        Self {
            next: ptr::null_mut(),
        }
    }

    pub(crate) fn containing_record(&self) -> &E {
        unsafe { &*(self.element_address() as *const E) }
    }

    pub(crate) fn containing_record_mut(&mut self) -> &mut E {
        unsafe { &mut *(self.element_address() as *mut E) }
    }

    fn element_address(&self) -> usize {
        self as *const _ as usize - E::offset()
    }
}

impl<E, L> Default for NtSingleListEntry<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    fn default() -> Self {
        Self::new()
    }
}
