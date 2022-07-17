// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::pin::Pin;

use moveit::{new, New};

use super::base::{Iter, IterMut, NtListEntry, NtListHead};
use super::traits::{NtBoxedListElement, NtList, NtListElement};

/// A variant of [`NtListHead`] that boxes every element on insertion.
/// This guarantees ownership and therefore all `NtBoxingListHead` functions can be used without
/// resorting to `unsafe`.
///
/// You need to implement the [`NtBoxedListElement`] trait to designate a single list as the boxing one.
/// This also establishes clear ownership when a single element is part of more than one list.
#[repr(transparent)]
pub struct NtBoxingListHead<E: NtBoxedListElement<L = L> + NtListElement<L>, L: NtList>(
    NtListHead<E, L>,
);

impl<E, L> NtBoxingListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtList,
{
    /// This function substitutes `InitializeListHead` of the Windows NT API.
    pub fn new() -> impl New<Output = Self> {
        unsafe {
            new::of(Self(NtListHead {
                flink: MaybeUninit::uninit().assume_init(),
                blink: MaybeUninit::uninit().assume_init(),
                pin: PhantomPinned,
            }))
            .with(|this| {
                let this = this.get_unchecked_mut();
                this.0.flink = this as *mut _ as usize as *mut NtListEntry<E, L>;
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

    fn inner(self: Pin<&Self>) -> Pin<&NtListHead<E, L>> {
        unsafe { Pin::new_unchecked(&self.get_ref().0) }
    }

    fn inner_mut(self: Pin<&mut Self>) -> Pin<&mut NtListHead<E, L>> {
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
                let entry = NtListHead::entry(element);

                unsafe {
                    (*entry).remove();
                    Box::from_raw(element);
                }
            }
        }
    }
}

impl<E, L> Drop for NtBoxingListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtList,
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

#[cfg(test)]
mod tests {
    use super::*;
    use memoffset::offset_of;
    use moveit::moveit;

    enum MyList {}
    impl NtList for MyList {}

    #[derive(Default)]
    #[repr(C)]
    struct TestItem {
        value: i32,
        entry: NtListEntry<Self, MyList>,
    }

    impl TestItem {
        fn new(value: i32) -> Self {
            Self {
                value,
                ..Default::default()
            }
        }
    }

    impl NtListElement<MyList> for TestItem {
        fn offset() -> usize {
            offset_of!(TestItem, entry)
        }
    }

    impl NtBoxedListElement for TestItem {
        type L = MyList;
    }

    #[test]
    fn test_append() {
        // Append two lists of equal size.
        moveit! {
            let mut list1 = NtBoxingListHead::<TestItem, MyList>::new();
            let mut list2 = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..10 {
            list1.as_mut().push_back(TestItem::new(i));
            list2.as_mut().push_back(TestItem::new(i));
        }

        list1.as_mut().append(list2.as_mut());

        assert_eq!(list1.as_ref().len(), 20);
        assert_eq!(list2.as_ref().len(), 0);

        for (i, element) in (0..10).chain(0..10).zip(list1.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list1.as_ref().inner());

        // Append the final list to an empty list.
        moveit! {
            let mut list3 = NtBoxingListHead::<TestItem, MyList>::new();
        }

        list3.as_mut().append(list1.as_mut());

        assert_eq!(list3.as_ref().len(), 20);
        assert_eq!(list1.as_ref().len(), 0);

        verify_all_links(list3.as_ref().inner());
    }

    #[test]
    fn test_back_and_front() {
        moveit! {
            let mut list = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..=3 {
            list.as_mut().push_back(TestItem::new(i));
        }

        assert_eq!(list.as_ref().back().unwrap().value, 3);
        assert_eq!(list.as_mut().back_mut().unwrap().value, 3);
        assert_eq!(list.as_ref().front().unwrap().value, 0);
        assert_eq!(list.as_mut().front_mut().unwrap().value, 0);
    }

    #[test]
    fn test_pop_back() {
        moveit! {
            let mut list = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(TestItem::new(i));
        }

        for i in (0..10).rev() {
            let element = list.as_mut().pop_back().unwrap();
            assert_eq!(i, element.value);
            verify_all_links(list.as_ref().inner());
        }

        assert!(list.as_ref().is_empty());
    }

    #[test]
    fn test_pop_front() {
        moveit! {
            let mut list = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(TestItem::new(i));
        }

        for i in 0..10 {
            let element = list.as_mut().pop_front().unwrap();
            assert_eq!(i, element.value);
            verify_all_links(list.as_ref().inner());
        }

        assert!(list.as_ref().is_empty());
    }

    #[test]
    fn test_push_back() {
        moveit! {
            let mut list = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(TestItem::new(i));
        }

        assert_eq!(list.as_ref().len(), 10);

        for (i, element) in (0..10).zip(list.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list.as_ref().inner());
    }

    #[test]
    fn test_push_front() {
        moveit! {
            let mut list = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_front(TestItem::new(i));
        }

        assert_eq!(list.as_ref().len(), 10);

        for (i, element) in (0..10).rev().zip(list.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list.as_ref().inner());
    }

    #[test]
    fn test_retain() {
        moveit! {
            let mut list = NtBoxingListHead::<TestItem, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(TestItem::new(i));
        }

        // Retain all even elements.
        list.as_mut().retain(|element| element.value % 2 == 0);

        assert_eq!(list.as_ref().len(), 5);

        for (i, element) in (0..10).step_by(2).zip(list.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list.as_ref().inner());
    }

    fn verify_all_links<E, L>(head: Pin<&NtListHead<E, L>>)
    where
        E: NtListElement<L>,
        L: NtList,
    {
        let mut current;
        let end = head.get_ref() as *const _ as usize as *mut NtListEntry<E, L>;

        // Traverse the list in forward direction and collect all entries.
        current = head.flink;
        let mut forward_entries = Vec::<*mut NtListEntry<E, L>>::new();

        while current != end {
            if !forward_entries.is_empty() {
                // Verify that the previous entry is referenced by this entry's `blink`.
                unsafe {
                    assert_eq!(*forward_entries.last().unwrap(), (*current).blink);
                }
            }

            forward_entries.push(current);
            current = unsafe { (*current).flink };
        }

        // Traverse the list in backward direction and collect all entries.
        current = head.blink;
        let mut backward_entries =
            Vec::<*mut NtListEntry<E, L>>::with_capacity(forward_entries.len());

        while current != end {
            if !backward_entries.is_empty() {
                // Verify that the previous entry is referenced by this entry's `flink`.
                unsafe {
                    assert_eq!(*backward_entries.last().unwrap(), (*current).flink);
                }
            }

            backward_entries.push(current);
            current = unsafe { (*current).blink };
        }

        // Verify that `backward_entries` is the exact reverse of `forward_entries`.
        assert_eq!(forward_entries.len(), backward_entries.len());

        for (fe, be) in forward_entries.iter().zip(backward_entries.iter().rev()) {
            assert_eq!(fe, be);
        }
    }
}
