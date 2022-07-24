// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::ptr;

use super::base::{Iter, IterMut, NtSingleListEntry, NtSingleListHead};
use super::traits::NtSingleList;
use crate::traits::{NtBoxedListElement, NtListElement, NtListOfType};

/// A variant of [`NtSingleListHead`] that boxes every element on insertion.
/// This guarantees ownership and therefore all `NtBoxingSingleListHead` functions can be used without
/// resorting to `unsafe`.
///
/// You need to implement the [`NtBoxedListElement`] trait to designate a single list as the boxing one.
/// This also establishes clear ownership when a single element is part of more than one list.
#[repr(transparent)]
pub struct NtBoxingSingleListHead<
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtListOfType<T = NtSingleList>,
>(NtSingleListHead<E, L>);

impl<E, L> NtBoxingSingleListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtListOfType<T = NtSingleList>,
{
    pub fn new() -> Self {
        Self(NtSingleListHead::<E, L>::new())
    }

    /// This operation computes in *O*(*n*) time.
    pub fn clear(&mut self) {
        for element in self.iter_mut() {
            unsafe {
                Box::from_raw(element);
            }
        }

        self.0.clear();
    }

    /// This operation computes in *O*(*1*) time.
    pub fn front(&self) -> Option<&E> {
        unsafe { self.0.front() }
    }

    /// This operation computes in *O*(*1*) time.
    pub fn front_mut(&mut self) -> Option<&mut E> {
        unsafe { self.0.front_mut() }
    }

    /// This operation computes in *O*(*1*) time.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<E, L> {
        unsafe { self.0.iter() }
    }

    pub fn iter_mut(&mut self) -> IterMut<E, L> {
        unsafe { self.0.iter_mut() }
    }

    /// This operation computes in *O*(*n*) time.
    pub fn len(&self) -> usize {
        unsafe { self.0.len() }
    }

    /// This function substitutes `PopEntryList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn pop_front(&mut self) -> Option<Box<E>> {
        unsafe { self.0.pop_front().map(|element| Box::from_raw(element)) }
    }

    /// This function substitutes `PushEntryList` of the Windows NT API.
    ///
    /// This operation computes in *O*(*1*) time.
    pub fn push_front(&mut self, element: E) {
        let boxed_element = Box::new(element);
        unsafe { self.0.push_front(Box::leak(boxed_element)) }
    }

    /// This operation computes in *O*(*n*) time.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut E) -> bool,
    {
        let mut previous = self as *mut _ as usize as *mut NtSingleListEntry<E, L>;
        let mut current = self.0.next;

        while current != ptr::null_mut() {
            unsafe {
                let element = (&*current).containing_record_mut();

                if f(element) {
                    previous = current;
                } else {
                    (*previous).next = (*current).next;
                    Box::from_raw(element);
                }

                current = (*current).next;
            }
        }
    }
}

impl<E, L> Drop for NtBoxingSingleListHead<E, L>
where
    E: NtBoxedListElement<L = L> + NtListElement<L>,
    L: NtListOfType<T = NtSingleList>,
{
    fn drop(&mut self) {
        for element in self.iter_mut() {
            // Reconstruct the `Box` we created in push_front and let it leave the scope
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
