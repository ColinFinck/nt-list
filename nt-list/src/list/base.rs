// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::pin::Pin;

use moveit::{new, New};

use super::traits::NtList;
use crate::traits::{NtListElement, NtListOfType};

/// This structure substitutes the `LIST_ENTRY` structure of the Windows NT API for the list header.
#[repr(C)]
pub struct NtListHead<E: NtListElement<L>, L: NtListOfType<T = NtList>> {
    pub(crate) flink: *mut NtListEntry<E, L>,
    pub(crate) blink: *mut NtListEntry<E, L>,
    pub(crate) pin: PhantomPinned,
}

impl<E, L> NtListHead<E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
    pub fn new() -> impl New<Output = Self> {
        unsafe {
            new::of(Self {
                flink: MaybeUninit::uninit().assume_init(),
                blink: MaybeUninit::uninit().assume_init(),
                pin: PhantomPinned,
            })
            .with(|this| {
                let this = this.get_unchecked_mut();
                this.flink = this as *mut _ as usize as *mut NtListEntry<E, L>;
                this.blink = this.flink;
            })
        }
    }

    pub unsafe fn append(self: Pin<&mut Self>, other: Pin<&mut Self>) {
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
        (*other.blink).flink = self.as_ref().end_marker();
        self.get_unchecked_mut().blink = other.blink;

        // Clear `other` without touching any of its elements.
        let other_end_marker = other.as_ref().end_marker();
        let other_mut = other.get_unchecked_mut();
        other_mut.flink = other_end_marker;
        other_mut.blink = other_end_marker;
    }

    pub unsafe fn back(self: Pin<&Self>) -> Option<&E> {
        (!self.is_empty()).then(|| (&*self.blink).containing_record())
    }

    pub unsafe fn back_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| (&mut *self.blink).containing_record_mut())
    }

    pub fn clear(self: Pin<&mut Self>) {
        let end_marker = self.as_ref().end_marker();
        let self_mut = unsafe { self.get_unchecked_mut() };

        self_mut.flink = end_marker;
        self_mut.blink = end_marker;
    }

    /// Returns the "end marker element" (which is the address of our own `NtListHead`, but interpreted as a `NtListEntry` element address).
    fn end_marker(self: Pin<&Self>) -> *mut NtListEntry<E, L> {
        self.get_ref() as *const _ as usize as *mut NtListEntry<E, L>
    }

    /// Returns the [`NtListEntry`] for the given element.
    pub(crate) fn entry(element: &mut E) -> *mut NtListEntry<E, L> {
        let element_address = element as *mut _ as usize;
        let entry_address = element_address + E::offset();
        entry_address as *mut NtListEntry<E, L>
    }

    pub unsafe fn front(self: Pin<&Self>) -> Option<&E> {
        (!self.is_empty()).then(|| (&*self.flink).containing_record())
    }

    pub unsafe fn front_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| (&mut *self.flink).containing_record_mut())
    }

    pub fn is_empty(self: Pin<&Self>) -> bool {
        self.flink as usize == self.get_ref() as *const _ as usize
    }

    pub unsafe fn iter(self: Pin<&Self>) -> Iter<E, L> {
        let head = self.get_ref();
        let flink = head.flink;
        let blink = head.blink;

        Iter { head, flink, blink }
    }

    pub unsafe fn iter_mut(self: Pin<&mut Self>) -> IterMut<E, L> {
        let head = self.get_unchecked_mut();
        let flink = head.flink;
        let blink = head.blink;

        IterMut { head, flink, blink }
    }

    pub unsafe fn len(self: Pin<&Self>) -> usize {
        self.iter().count()
    }

    pub unsafe fn pop_back(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| {
            let entry = &mut *self.blink;
            entry.remove();
            entry.containing_record_mut()
        })
    }

    pub unsafe fn pop_front(self: Pin<&mut Self>) -> Option<&mut E> {
        (!self.as_ref().is_empty()).then(|| {
            let entry = &mut *self.flink;
            entry.remove();
            entry.containing_record_mut()
        })
    }

    pub unsafe fn push_back(self: Pin<&mut Self>, element: &mut E) {
        let entry = Self::entry(element);

        let old_blink = self.blink;
        (*entry).flink = self.as_ref().end_marker();
        (*entry).blink = old_blink;
        (*old_blink).flink = entry;
        self.get_unchecked_mut().blink = entry;
    }

    pub unsafe fn push_front(self: Pin<&mut Self>, element: &mut E) {
        let entry = Self::entry(element);

        let old_flink = self.flink;
        (*entry).flink = old_flink;
        (*entry).blink = self.as_ref().end_marker();
        (*old_flink).blink = entry;
        self.get_unchecked_mut().flink = entry;
    }

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

pub struct Iter<'a, E: NtListElement<L>, L: NtListOfType<T = NtList>> {
    head: &'a NtListHead<E, L>,
    flink: *const NtListEntry<E, L>,
    blink: *const NtListEntry<E, L>,
}

impl<'a, E, L> Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
    fn terminate(&mut self) {
        self.flink = self.head as *const _ as usize as *const NtListEntry<E, L>;
        self.blink = self.flink;
    }
}

impl<'a, E, L> Iterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
    type Item = &'a E;

    fn next(&mut self) -> Option<&'a E> {
        if self.flink as usize == self.head as *const _ as usize {
            None
        } else {
            unsafe {
                let element = (&*self.flink).containing_record();

                if self.flink == self.blink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.flink = (&*self.flink).flink;
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
    L: NtListOfType<T = NtList>,
{
    fn next_back(&mut self) -> Option<&'a E> {
        if self.blink as usize == self.head as *const _ as usize {
            None
        } else {
            unsafe {
                let element = (&*self.blink).containing_record();

                if self.blink == self.flink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.blink = (&*self.blink).blink;
                }

                Some(element)
            }
        }
    }
}

impl<'a, E, L> FusedIterator for Iter<'a, E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
}

pub struct IterMut<'a, E: NtListElement<L>, L: NtListOfType<T = NtList>> {
    head: &'a mut NtListHead<E, L>,
    flink: *mut NtListEntry<E, L>,
    blink: *mut NtListEntry<E, L>,
}

impl<'a, E, L> IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
    fn terminate(&mut self) {
        self.flink = self.head as *const _ as usize as *mut NtListEntry<E, L>;
        self.blink = self.flink;
    }
}

impl<'a, E, L> Iterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
    type Item = &'a mut E;

    fn next(&mut self) -> Option<&'a mut E> {
        if self.flink as usize == self.head as *const _ as usize {
            None
        } else {
            unsafe {
                let element = (&*self.flink).containing_record_mut();

                if self.flink == self.blink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.flink = (&*self.flink).flink;
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
    L: NtListOfType<T = NtList>,
{
    fn next_back(&mut self) -> Option<&'a mut E> {
        if self.blink as usize == self.head as *const _ as usize {
            None
        } else {
            unsafe {
                let element = (&*self.blink).containing_record_mut();

                if self.blink == self.flink {
                    // We are crossing the other end of the iterator and must not iterate any further.
                    self.terminate();
                } else {
                    self.blink = (&*self.blink).blink;
                }

                Some(element)
            }
        }
    }
}

impl<'a, E, L> FusedIterator for IterMut<'a, E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
}

/// This structure substitutes the `LIST_ENTRY` structure of the Windows NT API for actual list entries.
#[derive(Debug)]
#[repr(C)]
pub struct NtListEntry<E: NtListElement<L>, L: NtListOfType<T = NtList>> {
    pub(crate) flink: *mut NtListEntry<E, L>,
    pub(crate) blink: *mut NtListEntry<E, L>,
    pin: PhantomPinned,
}

impl<E, L> NtListEntry<E, L>
where
    E: NtListElement<L>,
    L: NtListOfType<T = NtList>,
{
    pub fn new() -> Self {
        unsafe {
            Self {
                flink: MaybeUninit::uninit().assume_init(),
                blink: MaybeUninit::uninit().assume_init(),
                pin: PhantomPinned,
            }
        }
    }

    fn containing_record(&self) -> &E {
        unsafe { &*(self.element_address() as *const E) }
    }

    fn containing_record_mut(&self) -> &mut E {
        unsafe { &mut *(self.element_address() as *mut E) }
    }

    fn element_address(&self) -> usize {
        self as *const _ as usize - E::offset()
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
    L: NtListOfType<T = NtList>,
{
    fn default() -> Self {
        Self::new()
    }
}
