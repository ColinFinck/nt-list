// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::pin::Pin;

use moveit::{new, New};

use super::base::{Iter, IterMut, ListEntry, ListHead};
use super::traits::{BoxedListEntry, HasListEntry, IsDoublyLinkedList};

/// A variant of [`ListHead`] that boxes every element on insertion.
/// This guarantees ownership and therefore all `BoxingListHead` functions can be used without
/// resorting to `unsafe`.
///
/// You need to implement the [`BoxedListEntry`] trait to designate a single list as the boxing one.
/// This also establishes clear ownership when a single element is part of more than one list.
#[repr(transparent)]
pub struct BoxingListHead<E: BoxedListEntry<L = L> + HasListEntry<L>, L: IsDoublyLinkedList>(
    ListHead<E, L>,
);

impl<E, L> BoxingListHead<E, L>
where
    E: BoxedListEntry<L = L> + HasListEntry<L>,
    L: IsDoublyLinkedList,
{
    /// This function substitutes `InitializeListHead` of the Windows NT API.
    pub fn new() -> impl New<Output = Self> {
        unsafe {
            new::of(Self(ListHead {
                flink: MaybeUninit::uninit().assume_init(),
                blink: MaybeUninit::uninit().assume_init(),
                pin: PhantomPinned,
            }))
            .with(|this| {
                let this = this.get_unchecked_mut();
                this.0.flink = this as *mut _ as usize as *mut ListEntry<E, L>;
                this.0.blink = this.0.flink;
            })
        }
    }

    /// This operation computes in *O*(*1*) time.
    pub fn append(self: Pin<&mut Self>, other: Pin<&mut Self>) {
        unsafe { self.inner_mut().append(other.inner_mut()) }
    }

    /// This operation computes in *O*(*1*) time.
    pub fn back(self: Pin<&Self>) -> Option<&E> {
        unsafe { self.inner().back() }
    }

    /// This operation computes in *O*(*1*) time.
    pub fn back_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        unsafe { self.inner_mut().back_mut() }
    }

    /// This operation computes in *O*(*n*) time.
    pub fn clear(self: Pin<&mut Self>) {
        self.retain(|_| false)
    }

    /// This operation computes in *O*(*1*) time.
    pub fn front(self: Pin<&Self>) -> Option<&E> {
        unsafe { self.inner().front() }
    }

    /// This operation computes in *O*(*1*) time.
    pub fn front_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        unsafe { self.inner_mut().front_mut() }
    }

    fn inner(self: Pin<&Self>) -> Pin<&ListHead<E, L>> {
        unsafe { Pin::new_unchecked(&self.get_ref().0) }
    }

    fn inner_mut(self: Pin<&mut Self>) -> Pin<&mut ListHead<E, L>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0) }
    }

    /// This function substitutes `IsListEmpty` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn is_empty(self: Pin<&Self>) -> bool {
        self.inner().is_empty()
    }

    pub fn iter(self: Pin<&Self>) -> Iter<E, L> {
        unsafe { self.inner().iter() }
    }

    pub fn iter_mut(self: Pin<&mut Self>) -> IterMut<E, L> {
        unsafe { self.inner_mut().iter_mut() }
    }

    /// This operation computes in *O*(*n*) time.
    pub fn len(self: Pin<&Self>) -> usize {
        unsafe { self.inner().len() }
    }

    /// This function substitutes `RemoveTailList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn pop_back(self: Pin<&mut Self>) -> Option<Box<E>> {
        unsafe {
            self.inner_mut()
                .pop_back()
                .map(|element| Box::from_raw(element))
        }
    }

    /// This function substitutes `RemoveHeadList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn pop_front(self: Pin<&mut Self>) -> Option<Box<E>> {
        unsafe {
            self.inner_mut()
                .pop_front()
                .map(|element| Box::from_raw(element))
        }
    }

    /// This function substitutes `InsertTailList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn push_back(self: Pin<&mut Self>, element: E) {
        let boxed_element = Box::new(element);
        unsafe { self.inner_mut().push_back(Box::leak(boxed_element)) }
    }

    /// This function substitutes `InsertHeadList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn push_front(self: Pin<&mut Self>, element: E) {
        let boxed_element = Box::new(element);
        unsafe { self.inner_mut().push_front(Box::leak(boxed_element)) }
    }

    /// This function substitutes `RemoveEntryList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*n*) time.
    pub fn retain<F>(self: Pin<&mut Self>, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        for element in self.iter_mut() {
            if !f(element) {
                let entry = ListHead::entry(element);

                unsafe {
                    (*entry).remove();
                    Box::from_raw(element);
                }
            }
        }
    }
}

impl<E, L> Drop for BoxingListHead<E, L>
where
    E: BoxedListEntry<L = L> + HasListEntry<L>,
    L: IsDoublyLinkedList,
{
    fn drop(&mut self) {
        let pinned = unsafe { Pin::new_unchecked(self) };

        for element in pinned.iter_mut() {
            // Reconstruct the `Box` we created in push_back/push_front and let it leave the scope
            // to call its Drop handler and deallocate the element gracefully.
            unsafe {
                Box::from_raw(element);
            }
        }
    }
}
