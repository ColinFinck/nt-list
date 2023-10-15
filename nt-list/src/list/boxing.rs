// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::marker::PhantomPinned;
use core::pin::Pin;
use core::ptr;

use alloc::boxed::Box;
use moveit::{new, New};

use super::base::{Iter, IterMut, NtListEntry, NtListHead};
use super::traits::NtList;
use crate::traits::{NtBoxedListElement, NtListElement, NtTypedList};

/// A variant of [`NtListHead`] that boxes every element on insertion.
///
/// This guarantees ownership and therefore all `NtBoxingListHead` functions can be used without
/// resorting to `unsafe`.
/// If you can, use this implementation over [`NtListHead`].
///
/// You need to implement the [`NtBoxedListElement`] trait to designate a single list as the boxing one.
/// This also establishes clear ownership when a single element is part of more than one list.
///
/// See the [module-level documentation](crate::list) for more details.
///
/// This structure substitutes the [`LIST_ENTRY`] structure of the Windows NT API for the list header.
///
/// [`LIST_ENTRY`]: https://docs.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-list_entry
#[repr(transparent)]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub struct NtBoxingListHead<
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtList>,
>(NtListHead<E, L>);

impl<E, L> NtBoxingListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    /// Creates a new doubly linked list that owns all elements.
    ///
    /// This function substitutes [`InitializeListHead`] of the Windows NT API.
    ///
    /// [`InitializeListHead`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-initializelisthead
    pub fn new() -> impl New<Output = Self> {
        new::of(Self(NtListHead {
            flink: ptr::null_mut(),
            blink: ptr::null_mut(),
            pin: PhantomPinned,
        }))
        .with(|this| {
            let this = unsafe { this.get_unchecked_mut() };
            this.0.flink = (this as *mut Self).cast();
            this.0.blink = this.0.flink;
        })
    }

    /// Moves all elements from `other` to the end of the list.
    ///
    /// This reuses all the nodes from `other` and moves them into `self`.
    /// After this operation, `other` becomes empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn append(self: Pin<&mut Self>, other: Pin<&mut Self>) {
        unsafe { self.inner_mut().append(other.inner_mut()) }
    }

    /// Provides a reference to the last element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn back(self: Pin<&Self>) -> Option<&E> {
        unsafe { self.inner().back() }
    }

    /// Provides a mutable reference to the last element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn back_mut(self: Pin<&mut Self>) -> Option<&mut E> {
        unsafe { self.inner_mut().back_mut() }
    }

    /// Removes all elements from the list, deallocating their memory.
    ///
    /// Unlike [`NtListHead::clear`], this operation computes in *O*(*n*) time, because it
    /// needs to traverse all elements to deallocate them.
    pub fn clear(mut self: Pin<&mut Self>) {
        let end_marker = self.as_mut().inner_mut().end_marker_mut();

        // Get the link to the first element before it's being reset.
        let mut current = self.0.flink;

        // Make the list appear empty before deallocating any element.
        // By doing this here and not at the very end, we guard against the following scenario:
        //
        // 1. We deallocate an element.
        // 2. The `Drop` handler of that element is called and panics.
        // 3. Consequently, the `Drop` handler of `NtBoxingListHead` is called and removes all elements.
        // 4. While removing elements, the just dropped element is dropped again.
        //
        // By clearing the list at the beginning, the `Drop` handler of `NtBoxingListHead` won't find any
        // elements, and thereby it won't drop any elements.
        self.inner_mut().clear();

        // Traverse the list in the old-fashioned way and deallocate each element.
        while current != end_marker {
            unsafe {
                let element = NtListEntry::containing_record_mut(current);
                current = (*current).flink;
                drop(Box::from_raw(element));
            }
        }
    }

    /// Provides a reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn front(self: Pin<&Self>) -> Option<&E> {
        unsafe { self.inner().front() }
    }

    /// Provides a mutable reference to the first element, or `None` if the list is empty.
    ///
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

    /// Returns `true` if the list is empty.
    ///
    /// This function substitutes [`IsListEmpty`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`IsListEmpty`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-islistempty
    pub fn is_empty(self: Pin<&Self>) -> bool {
        self.inner().is_empty()
    }

    /// Returns an iterator yielding references to each element of the list.
    pub fn iter(self: Pin<&Self>) -> Iter<E, L> {
        unsafe { self.inner().iter() }
    }

    /// Returns an iterator yielding mutable references to each element of the list.
    pub fn iter_mut(self: Pin<&mut Self>) -> IterMut<E, L> {
        unsafe { self.inner_mut().iter_mut() }
    }

    /// Counts all elements and returns the length of the list.
    ///
    /// This operation computes in *O*(*n*) time.
    pub fn len(self: Pin<&Self>) -> usize {
        unsafe { self.inner().len() }
    }

    /// Removes the last element from the list and returns it, or `None` if the list is empty.
    ///
    /// This function substitutes [`RemoveTailList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`RemoveTailList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-removetaillist
    pub fn pop_back(self: Pin<&mut Self>) -> Option<Box<E>> {
        unsafe {
            self.inner_mut()
                .pop_back()
                .map(|element| Box::from_raw(element))
        }
    }

    /// Removes the first element from the list and returns it, or `None` if the list is empty.
    ///
    /// This function substitutes [`RemoveHeadList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`RemoveHeadList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-removeheadlist
    pub fn pop_front(self: Pin<&mut Self>) -> Option<Box<E>> {
        unsafe {
            self.inner_mut()
                .pop_front()
                .map(|element| Box::from_raw(element))
        }
    }

    /// Appends an element to the back of the list.
    ///
    /// This function substitutes [`InsertTailList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`InsertTailList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-inserttaillist
    pub fn push_back(self: Pin<&mut Self>, element: E) {
        let boxed_element = Box::new(element);
        unsafe { self.inner_mut().push_back(Box::leak(boxed_element)) }
    }

    /// Appends an element to the front of the list.
    ///
    /// This function substitutes [`InsertHeadList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`InsertHeadList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-insertheadlist
    pub fn push_front(self: Pin<&mut Self>, element: E) {
        let boxed_element = Box::new(element);
        unsafe { self.inner_mut().push_front(Box::leak(boxed_element)) }
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
    pub fn retain<F>(self: Pin<&mut Self>, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        for element in self.iter_mut() {
            if !f(element) {
                let entry = NtListHead::entry(element);

                unsafe {
                    (*entry).remove();
                    drop(Box::from_raw(element));
                }
            }
        }
    }
}

impl<E, L> Drop for NtBoxingListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn drop(&mut self) {
        let pinned = unsafe { Pin::new_unchecked(self) };

        for element in pinned.iter_mut() {
            // Reconstruct the `Box` we created in push_back/push_front and let it leave the scope
            // to call its Drop handler and deallocate the element gracefully.
            unsafe {
                drop(Box::from_raw(element));
            }
        }
    }
}

impl<E, L> Extend<Box<E>> for Pin<&mut NtBoxingListHead<E, L>>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Box<E>>,
    {
        let end_marker = self.as_mut().inner_mut().end_marker_mut();
        let mut previous = self.as_ref().inner().blink;

        for element in iter.into_iter() {
            // We could use `NtBoxingListHead::push_back` here, but this manual implementation
            // is slightly optimized (doesn't modify list head's `blink` on every iteration).
            unsafe {
                let entry = NtListHead::entry(Box::leak(element));

                (*entry).flink = end_marker;
                (*entry).blink = previous;
                (*previous).flink = entry;

                previous = entry;
            }
        }

        unsafe {
            self.as_mut().get_unchecked_mut().0.blink = previous;
        }
    }
}

impl<E, L> Extend<E> for Pin<&mut NtBoxingListHead<E, L>>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtList>,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = E>,
    {
        self.extend(iter.into_iter().map(Box::new))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::list::NtListEntry;
    use alloc::vec::Vec;
    use moveit::moveit;

    #[derive(NtList)]
    enum MyList {}

    #[derive(Default, NtListElement)]
    #[repr(C)]
    struct MyElement {
        value: i32,
        #[boxed]
        entry: NtListEntry<Self, MyList>,
    }

    impl MyElement {
        fn new(value: i32) -> Self {
            Self {
                value,
                ..Default::default()
            }
        }
    }

    #[test]
    fn test_append() {
        // Append two lists of equal size.
        moveit! {
            let mut list1 = NtBoxingListHead::<MyElement, MyList>::new();
            let mut list2 = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list1.as_mut().push_back(MyElement::new(i));
            list2.as_mut().push_back(MyElement::new(i));
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
            let mut list3 = NtBoxingListHead::<MyElement, MyList>::new();
        }

        list3.as_mut().append(list1.as_mut());

        assert_eq!(list3.as_ref().len(), 20);
        assert_eq!(list1.as_ref().len(), 0);

        verify_all_links(list3.as_ref().inner());
    }

    #[test]
    fn test_clear_and_append() {
        // Append two lists of equal size.
        moveit! {
            let mut list1 = NtBoxingListHead::<MyElement, MyList>::new();
            let mut list2 = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list1.as_mut().push_back(MyElement::new(i));
            list2.as_mut().push_back(MyElement::new(i));
        }

        list1.as_mut().append(list2.as_mut());

        assert_eq!(list1.as_ref().len(), 20);
        assert_eq!(list2.as_ref().len(), 0);

        for (i, element) in (0..10).chain(0..10).zip(list1.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list1.as_ref().inner());

        // Add more elements to both lists
        list1.as_mut().push_back(MyElement::new(21));
        list1.as_mut().push_front(MyElement::new(22));

        list2.as_mut().push_back(MyElement::new(21));
        list2.as_mut().push_front(MyElement::new(22));

        // Append the final list to a cleared list.
        moveit! {
            let mut list3 = NtBoxingListHead::<MyElement, MyList>::new();
        }

        list3.as_mut().clear();
        list3.as_mut().append(list1.as_mut());

        assert_eq!(list3.as_ref().len(), 22);
        assert_eq!(list1.as_ref().len(), 0);

        verify_all_links(list3.as_ref().inner());
    }

    #[test]
    fn test_clear_and_push() {
        moveit! {
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        list.as_mut().clear();

        for i in 0..=3 {
            list.as_mut().push_back(MyElement::new(i));
        }
        for i in 4..=6 {
            list.as_mut().push_front(MyElement::new(i));
        }

        assert_eq!(list.as_ref().back().unwrap().value, 3);
        assert_eq!(list.as_mut().back_mut().unwrap().value, 3);
        assert_eq!(list.as_ref().front().unwrap().value, 6);
        assert_eq!(list.as_mut().front_mut().unwrap().value, 6);

        verify_all_links(list.as_ref().inner());
    }

    #[test]
    fn test_back_and_front() {
        moveit! {
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..=3 {
            list.as_mut().push_back(MyElement::new(i));
        }

        assert_eq!(list.as_ref().back().unwrap().value, 3);
        assert_eq!(list.as_mut().back_mut().unwrap().value, 3);
        assert_eq!(list.as_ref().front().unwrap().value, 0);
        assert_eq!(list.as_mut().front_mut().unwrap().value, 0);
    }

    #[test]
    fn test_extend() {
        let integers = [0, 1, 2, 3, 4, 5];

        moveit! {
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        list.as_mut()
            .extend(integers.into_iter().map(MyElement::new));

        for (i, element) in integers.into_iter().zip(list.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list.as_ref().inner());
    }

    #[test]
    fn test_pop_back() {
        moveit! {
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(MyElement::new(i));
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
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(MyElement::new(i));
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
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(MyElement::new(i));
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
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_front(MyElement::new(i));
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
            let mut list = NtBoxingListHead::<MyElement, MyList>::new();
        }

        for i in 0..10 {
            list.as_mut().push_back(MyElement::new(i));
        }

        // Keep only the even elements.
        list.as_mut().retain(|element| element.value % 2 == 0);

        assert_eq!(list.as_ref().len(), 5);

        for (i, element) in (0..10).step_by(2).zip(list.as_ref().iter()) {
            assert_eq!(i, element.value);
        }

        verify_all_links(list.as_ref().inner());

        // Keep only the first and last of the remaining elements.
        list.as_mut()
            .retain(|element| element.value == 0 || element.value == 8);

        let mut iter = list.as_ref().iter();
        assert_eq!(iter.next().unwrap().value, 0);
        assert_eq!(iter.next().unwrap().value, 8);
        assert!(matches!(iter.next(), None));
    }

    fn verify_all_links<E, L>(head: Pin<&NtListHead<E, L>>)
    where
        E: NtListElement<L>,
        L: NtTypedList<T = NtList>,
    {
        let mut current;
        let end = (head.get_ref() as *const _ as *mut NtListHead<E, L>).cast();

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
