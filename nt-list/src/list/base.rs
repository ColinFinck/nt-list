// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::ptr;

use moveit::{new, New};

use super::traits::NtList;
use crate::traits::{NtListElement, NtTypedList};

/// A doubly linked list header compatible to [`LIST_ENTRY`] of the Windows NT API.
///
/// This variant requires elements to be allocated beforehand on a stable address and be
/// valid as long as the list is used.
/// As the Rust compiler cannot guarantee the validity of them, almost all `NtListHead`
/// functions are `unsafe`.
/// You almost always want to use [`NtBoxingListHead`] over this.
///
/// See the [module-level documentation](crate::list) for more details.
///
/// This structure substitutes the `LIST_ENTRY` structure of the Windows NT API for the list header.
///
/// [`LIST_ENTRY`]: https://docs.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-list_entry
/// [`NtBoxingListHead`]: crate::list::NtBoxingListHead
#[repr(C)]
pub struct NtListHead<E: NtListElement<L>, L: NtTypedList<T = NtList>> {
    pub(crate) flink: *mut NtListEntry<E, L>,
    pub(crate) blink: *mut NtListEntry<E, L>,
    pub(crate) pin: PhantomPinned,
}

impl<E, L> NtListHead<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    /// Creates a new doubly linked list.
    ///
    /// This function substitutes [`InitializeListHead`] of the Windows NT API.
    ///
    /// [`InitializeListHead`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-initializelisthead
    pub fn new() -> impl New<Output = Self> {
        new::of(Self {
            flink: ptr::null_mut(),
            blink: ptr::null_mut(),
            pin: PhantomPinned,
        })
        .with(|this| {
            let this = unsafe { this.get_unchecked_mut() };
            this.flink = (this as *mut Self).cast();
            this.blink = this.flink;
        })
    }

    /// Moves all elements from `other` to the end of the list.
    ///
    /// This reuses all the nodes from `other` and moves them into `self`.
    /// After this operation, `other` becomes empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn append(mut self: Pin<&mut Self>, mut other: Pin<&mut Self>) {
        if other.as_ref().is_empty() {
            return;
        }

        // Append `other` to `self` by remounting the respective elements:
        // - The last element of `self` shall be followed by the first element of `other`.
        // - The first element of `other` shall be preceded by the last element of `self`.
        // - The last element of `other` shall be followed by the end marker of `self`.
        // - The last element of `self` shall be changed to the last element of `other`.
        (*self.blink).flink = other.flink;
        (*other.flink).blink = self.blink;
        (*other.blink).flink = self.as_mut().end_marker_mut();
        self.get_unchecked_mut().blink = other.blink;

        // Clear `other` without touching any of its elements.
        let other_end_marker = other.as_mut().end_marker_mut();
        let other_mut = other.get_unchecked_mut();
        other_mut.flink = other_end_marker;
        other_mut.blink = other_end_marker;
    }

    /// Provides a reference to the last element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn back(self: Pin<&Self>) -> Option<&E> {
        (!self.is_empty()).then(|| (*self.blink).containing_record())
    }

    /// Provides a mutable reference to the last element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn back_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| (*self.blink).containing_record_mut())
    }

    /// Removes all elements from the list.
    ///
    /// This operation computes in *O*(*1*) time, because it only resets the forward and
    /// backward links of the header.
    pub fn clear(mut self: Pin<&mut Self>) {
        let end_marker = self.as_mut().end_marker_mut();
        let self_mut = unsafe { self.get_unchecked_mut() };

        self_mut.flink = end_marker;
        self_mut.blink = end_marker;
    }

    /// Returns a const pointer to the "end marker element" (which is the address of our own `NtListHead`, but interpreted as a `NtListEntry` element address).
    pub(crate) fn end_marker(self: Pin<&Self>) -> *const NtListEntry<E, L> {
        (self.get_ref() as *const _ as *mut Self).cast()
    }

    /// Returns a mutable pointer to the "end marker element" (which is the address of our own `NtListHead`, but interpreted as a `NtListEntry` element address).
    pub(crate) fn end_marker_mut(self: Pin<&mut Self>) -> *mut NtListEntry<E, L> {
        (unsafe { self.get_unchecked_mut() } as *mut Self).cast()
    }

    /// Returns the [`NtListEntry`] for the given element.
    pub(crate) fn entry(element: &mut E) -> *mut NtListEntry<E, L> {
        let element_ptr = element as *mut E;

        // This is the canonical implementation of `byte_add`
        let entry = unsafe { element_ptr.cast::<u8>().add(E::offset()).cast::<E>() };

        entry.cast()
    }

    /// Provides a reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn front(self: Pin<&Self>) -> Option<&E> {
        (!self.is_empty()).then(|| (*self.flink).containing_record())
    }

    /// Provides a mutable reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub unsafe fn front_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| (*self.flink).containing_record_mut())
    }

    /// Returns `true` if the list is empty.
    ///
    /// This function substitutes [`IsListEmpty`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`IsListEmpty`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-islistempty
    pub fn is_empty(self: Pin<&Self>) -> bool {
        self.flink as *const NtListEntry<E, L> == (self.get_ref() as *const Self).cast()
    }

    /// Returns an iterator yielding references to each element of the list.
    pub unsafe fn iter(self: Pin<&Self>) -> Iter<E, L> {
        let head = self.get_ref();
        let flink = head.flink;
        let blink = head.blink;

        Iter { head, flink, blink }
    }

    /// Returns an iterator yielding mutable references to each element of the list.
    pub unsafe fn iter_mut(self: Pin<&mut Self>) -> IterMut<E, L> {
        let head = self.get_unchecked_mut();
        let flink = head.flink;
        let blink = head.blink;

        IterMut { head, flink, blink }
    }

    /// Counts all elements and returns the length of the list.
    ///
    /// This operation computes in *O*(*n*) time.
    pub unsafe fn len(self: Pin<&Self>) -> usize {
        self.iter().count()
    }

    /// Removes the last element from the list and returns it, or `None` if the list is empty.
    ///
    /// This function substitutes [`RemoveTailList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`RemoveTailList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-removetaillist
    pub unsafe fn pop_back(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| {
            let entry = &mut *self.blink;
            entry.remove();
            entry.containing_record_mut()
        })
    }

    /// Removes the first element from the list and returns it, or `None` if the list is empty.
    ///
    /// This function substitutes [`RemoveHeadList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`RemoveHeadList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-removeheadlist
    pub unsafe fn pop_front(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| {
            let entry = &mut *self.flink;
            entry.remove();
            entry.containing_record_mut()
        })
    }

    /// Appends an element to the back of the list.
    ///
    /// This function substitutes [`InsertTailList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`InsertTailList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-inserttaillist
    pub unsafe fn push_back(mut self: Pin<&mut Self>, element: &mut E) {
        let entry = Self::entry(element);

        let old_blink = self.blink;
        (*entry).flink = self.as_mut().end_marker_mut();
        (*entry).blink = old_blink;
        (*old_blink).flink = entry;
        self.get_unchecked_mut().blink = entry;
    }

    /// Appends an element to the front of the list.
    ///
    /// This function substitutes [`InsertHeadList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`InsertHeadList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-insertheadlist
    pub unsafe fn push_front(mut self: Pin<&mut Self>, element: &mut E) {
        let entry = Self::entry(element);

        let old_flink = self.flink;
        (*entry).flink = old_flink;
        (*entry).blink = self.as_mut().end_marker_mut();
        (*old_flink).blink = entry;
        self.get_unchecked_mut().flink = entry;
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// In other words, remove all elements `e` for which `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the original order,
    /// and preserves the order of the retained elements.
    ///
    /// This function substitutes [`RemoveEntryList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*n*) time.
    ///
    /// [`RemoveEntryList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-removeentrylist
    pub unsafe fn retain<F>(self: Pin<&mut Self>, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        for element in self.iter_mut() {
            if !f(element) {
                let entry = Self::entry(element);
                (*entry).remove();
            }
        }
    }
}

/// Iterator over the elements of a doubly linked list.
///
/// This iterator is returned from the [`NtListHead::iter`] and [`NtBoxingListHead::iter`] functions.
///
/// [`NtBoxingListHead::iter`]: crate::list::NtBoxingListHead::iter
pub struct Iter<'a, E: NtListElement<L>, L: NtTypedList<T = NtList>> {
    head: &'a NtListHead<E, L>,
    flink: *const NtListEntry<E, L>,
    blink: *const NtListEntry<E, L>,
}

impl<'a, E, L> Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn terminate(&mut self) {
        self.flink = (self.head as *const NtListHead<E, L>).cast();
        self.blink = self.flink;
    }
}

impl<'a, E, L> Iterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    type Item = &'a E;

    fn next(&mut self) -> Option<&'a E> {
        if self.flink == (self.head as *const NtListHead<_, _>).cast() {
            None
        } else {
            unsafe {
                let element = (*self.flink).containing_record();

                if self.flink == self.blink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.flink = (*self.flink).flink;
                }

                Some(element)
            }
        }
    }

    fn last(mut self) -> Option<&'a E> {
        self.next_back()
    }
}

impl<'a, E, L> DoubleEndedIterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn next_back(&mut self) -> Option<&'a E> {
        if self.blink == (self.head as *const NtListHead<_, _>).cast() {
            None
        } else {
            unsafe {
                let element = (*self.blink).containing_record();

                if self.blink == self.flink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.blink = (*self.blink).blink;
                }

                Some(element)
            }
        }
    }
}

impl<'a, E, L> FusedIterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
}

/// Mutable iterator over the elements of a doubly linked list.
///
/// This iterator is returned from the [`NtListHead::iter_mut`] and [`NtBoxingListHead::iter_mut`] functions.
///
/// [`NtBoxingListHead::iter_mut`]: crate::list::NtBoxingListHead::iter_mut
pub struct IterMut<'a, E: NtListElement<L>, L: NtTypedList<T = NtList>> {
    head: &'a mut NtListHead<E, L>,
    flink: *mut NtListEntry<E, L>,
    blink: *mut NtListEntry<E, L>,
}

impl<'a, E, L> IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn terminate(&mut self) {
        self.flink = (self.head as *mut NtListHead<E, L>).cast();
        self.blink = self.flink;
    }
}

impl<'a, E, L> Iterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    type Item = &'a mut E;

    fn next(&mut self) -> Option<&'a mut E> {
        if self.flink == (self.head as *mut NtListHead<_, _>).cast() {
            None
        } else {
            unsafe {
                let element = (*self.flink).containing_record_mut();

                if self.flink == self.blink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.flink = (*self.flink).flink;
                }

                Some(element)
            }
        }
    }

    fn last(mut self) -> Option<&'a mut E> {
        self.next_back()
    }
}

impl<'a, E, L> DoubleEndedIterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn next_back(&mut self) -> Option<&'a mut E> {
        if self.blink == (self.head as *mut NtListHead<_, _>).cast() {
            None
        } else {
            unsafe {
                let element = (*self.blink).containing_record_mut();

                if self.blink == self.flink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.blink = (*self.blink).blink;
                }

                Some(element)
            }
        }
    }
}

impl<'a, E, L> FusedIterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
}

/// This structure substitutes the `LIST_ENTRY` structure of the Windows NT API for actual list entries.
#[derive(Debug)]
#[repr(C)]
pub struct NtListEntry<E: NtListElement<L>, L: NtTypedList<T = NtList>> {
    pub(crate) flink: *mut NtListEntry<E, L>,
    pub(crate) blink: *mut NtListEntry<E, L>,
    pin: PhantomPinned,
}

impl<E, L> NtListEntry<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    /// Allows the creation of an `NtListEntry`, but leaves all fields uninitialized.
    ///
    /// Its fields are only initialized when an entry is pushed to a list.
    pub fn new() -> Self {
        Self {
            flink: ptr::null_mut(),
            blink: ptr::null_mut(),
            pin: PhantomPinned,
        }
    }

    pub(crate) fn containing_record(&self) -> &E {
        unsafe { &*self.element_ptr() }
    }

    pub(crate) fn containing_record_mut(&mut self) -> &mut E {
        unsafe { &mut *self.element_ptr_mut() }
    }

    fn element_ptr(&self) -> *const E {
        let ptr = self as *const Self;

        // This is the canonical implementation of `byte_sub`
        let ptr = unsafe { ptr.cast::<u8>().sub(E::offset()).cast::<Self>() };

        ptr.cast()
    }

    fn element_ptr_mut(&mut self) -> *mut E {
        let ptr = self as *mut Self;

        // This is the canonical implementation of `byte_sub`
        let ptr = unsafe { ptr.cast::<u8>().sub(E::offset()).cast::<Self>() };

        ptr.cast()
    }

    pub(crate) unsafe fn remove(&mut self) {
        let old_flink = self.flink;
        let old_blink = self.blink;
        (*old_flink).blink = old_blink;
        (*old_blink).flink = old_flink;
    }
}

impl<E, L> Default for NtListEntry<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn default() -> Self {
        Self::new()
    }
}
