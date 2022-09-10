// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::boxed::Box;

use super::base::{Iter, IterMut, NtSingleListHead};
use super::traits::NtSingleList;
use crate::traits::{NtBoxedListElement, NtListElement, NtTypedList};

/// A variant of [`NtSingleListHead`] that boxes every element on insertion.
///
/// This guarantees ownership and therefore all `NtBoxingSingleListHead` functions can be used without
/// resorting to `unsafe`.
/// If you can, use this implementation over [`NtSingleListHead`].
///
/// You need to implement the [`NtBoxedListElement`] trait to designate a single list as the boxing one.
/// This also establishes clear ownership when a single element is part of more than one list.
///
/// See the [module-level documentation](crate::single_list) for more details.
///
/// This structure substitutes the [`SINGLE_LIST_ENTRY`] structure of the Windows NT API for the list header.
///
/// [`SINGLE_LIST_ENTRY`]: https://docs.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-single_list_entry
#[repr(transparent)]
pub struct NtBoxingSingleListHead<
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
>(NtSingleListHead<E, L>);

impl<E, L> NtBoxingSingleListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    /// Creates a new singly linked list that owns all elements.
    pub fn new() -> Self {
        Self(NtSingleListHead::<E, L>::new())
    }

    /// Removes all elements from the list, deallocating their memory.
    ///
    /// Unlike [`NtSingleListHead::clear`], this operation computes in *O*(*n*) time, because it
    /// needs to traverse all elements to deallocate them.
    pub fn clear(&mut self) {
        // Get the link to the first element before it's being reset.
        let mut current = self.0.next;

        // Make the list appear empty before deallocating any element.
        // By doing this here and not at the very end, we guard against the following scenario:
        //
        // 1. We deallocate an element.
        // 2. The `Drop` handler of that element is called and panics.
        // 3. Consequently, the `Drop` handler of `NtBoxingSingleListHead` is called and removes all elements.
        // 4. While removing elements, the just dropped element is dropped again.
        //
        // By clearing the list at the beginning, the `Drop` handler of `NtBoxingSingleListHead` won't find any
        // elements, and thereby it won't drop any elements.
        self.0.clear();

        // Traverse the list in the old-fashioned way and deallocate each element.
        while !current.is_null() {
            unsafe {
                let element = (&mut *current).containing_record_mut();
                current = (*current).next;
                drop(Box::from_raw(element));
            }
        }
    }

    /// Provides a reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn front(&self) -> Option<&E> {
        unsafe { self.0.front() }
    }

    /// Provides a mutable reference to the first element, or `None` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn front_mut(&mut self) -> Option<&mut E> {
        unsafe { self.0.front_mut() }
    }

    /// Returns `true` if the list is empty.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns an iterator yielding references to each element of the list.
    pub fn iter(&self) -> Iter<E, L> {
        unsafe { self.0.iter() }
    }

    /// Returns an iterator yielding mutable references to each element of the list.
    pub fn iter_mut(&mut self) -> IterMut<E, L> {
        unsafe { self.0.iter_mut() }
    }

    /// Counts all elements and returns the length of the list.
    ///
    /// This operation computes in *O*(*n*) time.
    pub fn len(&self) -> usize {
        unsafe { self.0.len() }
    }

    /// Removes the first element from the list and returns it, or `None` if the list is empty.
    ///
    /// This function substitutes [`PopEntryList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`PopEntryList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-popentrylist
    pub fn pop_front(&mut self) -> Option<Box<E>> {
        unsafe { self.0.pop_front().map(|element| Box::from_raw(element)) }
    }

    /// Appends an element to the front of the list.
    ///
    /// This function substitutes [`PushEntryList`] of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    ///
    /// [`PushEntryList`]: https://docs.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-pushentrylist
    pub fn push_front(&mut self, element: E) {
        let boxed_element = Box::new(element);
        unsafe { self.0.push_front(Box::leak(boxed_element)) }
    }

    /// Retains only the elements specified by the predicate, passing a mutable reference to it.
    ///
    /// In other words, remove all elements `e` for which `f(&mut e)` returns `false`.
    /// This method operates in place, visiting each element exactly once in the original order,
    /// and preserves the order of the retained elements.
    ///
    /// This operation computes in *O*(*n*) time.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        let mut previous = (self as *mut Self).cast();
        let mut current = self.0.next;

        while !current.is_null() {
            unsafe {
                let element = (&mut *current).containing_record_mut();

                if f(element) {
                    previous = current;
                    current = (*current).next;
                } else {
                    (*previous).next = (*current).next;
                    current = (*current).next;
                    drop(Box::from_raw(element));
                }
            }
        }
    }
}

impl<E, L> Default for NtBoxingSingleListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<E, L> Drop for NtBoxingSingleListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtTypedList<T = NtSingleList>,
{
    fn drop(&mut self) {
        for element in self.iter_mut() {
            // Reconstruct the `Box` we created in push_front and let it leave the scope
            // to call its Drop handler and deallocate the element gracefully.
            unsafe {
                drop(Box::from_raw(element));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::single_list::NtSingleListEntry;

    #[derive(NtSingleList)]
    enum MyList {}

    #[derive(Default, NtListElement)]
    #[repr(C)]
    struct MyElement {
        value: i32,
        #[boxed]
        entry: NtSingleListEntry<Self, MyList>,
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
    fn test_front() {
        let mut list = NtBoxingSingleListHead::<MyElement, MyList>::new();

        for i in 0..=3 {
            list.push_front(MyElement::new(i));
        }

        assert_eq!(list.front().unwrap().value, 3);
        assert_eq!(list.front_mut().unwrap().value, 3);
    }

    #[test]
    fn test_pop_front() {
        let mut list = NtBoxingSingleListHead::<MyElement, MyList>::new();

        for i in 0..10 {
            list.push_front(MyElement::new(i));
        }

        for i in (0..10).rev() {
            let element = list.pop_front().unwrap();
            assert_eq!(i, element.value);
        }

        assert!(list.is_empty());
    }

    #[test]
    fn test_push_front() {
        let mut list = NtBoxingSingleListHead::<MyElement, MyList>::new();

        for i in 0..10 {
            list.push_front(MyElement::new(i));
        }

        assert_eq!(list.len(), 10);

        for (i, element) in (0..10).rev().zip(list.iter()) {
            assert_eq!(i, element.value);
        }
    }

    #[test]
    fn test_retain() {
        let mut list = NtBoxingSingleListHead::<MyElement, MyList>::new();

        for i in 0..10 {
            list.push_front(MyElement::new(i));
        }

        // Keep only the even elements.
        list.retain(|element| element.value % 2 == 0);

        assert_eq!(list.len(), 5);

        for (i, element) in (0..=8).rev().step_by(2).zip(list.iter()) {
            assert_eq!(i, element.value);
        }

        // Keep only the first and last of the remaining elements.
        list.retain(|element| element.value == 8 || element.value == 0);

        let mut iter = list.iter();
        assert_eq!(iter.next().unwrap().value, 8);
        assert_eq!(iter.next().unwrap().value, 0);
        assert!(matches!(iter.next(), None));
    }
}
