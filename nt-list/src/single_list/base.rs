// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr;

use super::traits::NtSingleList;
use crate::traits::{NtListElement, NtTypedList};

/// This structure substitutes the `SINGLE_LIST_ENTRY` structure of the Windows NT API for the list header.
#[repr(C)]
pub struct NtSingleListHead<E: NtListElement<L>, L: NtTypedList<T = NtSingleList>> {
    pub(crate) next: *mut NtSingleListEntry<E, L>,
}

impl<E, L> NtSingleListHead<E, L>
where
    E: NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    pub fn new() -> Self {
        Self {
            next: ptr::null_mut(),
        }
    }

    pub fn clear(&mut self) {
        self.next = ptr::null_mut();
    }

    /// Returns the [`NtSingleListEntry`] for the given element.
    pub(crate) fn entry(element: &mut E) -> *mut NtSingleListEntry<E, L> {
        let element_address = element as *mut _ as usize;
        let entry_address = element_address + E::offset();
        entry_address as *mut NtSingleListEntry<E, L>
    }

    pub unsafe fn front(&self) -> Option<&E> {
        (!self.is_empty()).then(|| (&*self.next).containing_record())
    }

    pub unsafe fn front_mut(&mut self) -> Option<&mut E> {
        (!self.is_empty()).then(|| (&mut *self.next).containing_record_mut())
    }

    pub fn is_empty(&self) -> bool {
        self.next == ptr::null_mut()
    }

    pub unsafe fn iter(&self) -> Iter<E, L> {
        Iter {
            current: self.next,
            phantom: PhantomData,
        }
    }

    pub unsafe fn iter_mut(&mut self) -> IterMut<E, L> {
        IterMut {
            current: self.next,
            phantom: PhantomData,
        }
    }

    pub unsafe fn len(&self) -> usize {
        self.iter().count()
    }

    pub unsafe fn pop_front(&mut self) -> Option<&mut E> {
        (!self.is_empty()).then(|| {
            let entry = &mut *self.next;
            self.next = entry.next;
            entry.containing_record_mut()
        })
    }

    pub unsafe fn push_front(&mut self, element: &mut E) {
        let entry = Self::entry(element);

        (*entry).next = self.next;
        self.next = entry;
    }

    pub unsafe fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        let mut previous = self as *mut _ as usize as *mut NtSingleListEntry<E, L>;
        let mut current = self.next;

        while current != ptr::null_mut() {
            let element = (&*current).containing_record_mut();

            if f(element) {
                previous = current;
            } else {
                (*previous).next = (*current).next;
            }

            current = (*current).next;
        }
    }
}

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
        if self.current == ptr::null() {
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
        if self.current == ptr::null_mut() {
            None
        } else {
            unsafe {
                let element = (&*self.current).containing_record_mut();
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
    pub fn new() -> Self {
        unsafe {
            Self {
                next: MaybeUninit::uninit().assume_init(),
            }
        }
    }

    pub(crate) fn containing_record(&self) -> &E {
        unsafe { &*(self.element_address() as *const E) }
    }

    pub(crate) fn containing_record_mut(&self) -> &mut E {
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
