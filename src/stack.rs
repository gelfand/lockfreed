extern crate crossbeam;
use crossbeam::{
    epoch::{self, Atomic, Owned},
    utils::CachePadded,
};
use std::{ptr::NonNull, sync::atomic::Ordering};

#[derive(Debug)]
pub struct Stack<T> {
    top: Atomic<Node<T>>,
}

impl<T> Stack<T> {
    pub const fn new() -> Self {
        Self {
            top: Atomic::null(),
        }
    }
    pub fn push(&self, val: T) {
        let _guard = epoch::pin();

        let mut ptr = self.top.load(Ordering::Acquire, &_guard);
        let mut new = Owned::new(Node {
            val,
            next: ptr.as_raw() as *mut _,
        });
        loop {
            match self
                .top
                .compare_exchange(ptr, new, Ordering::AcqRel, Ordering::Acquire, &_guard)
            {
                Ok(_) => break,
                Err(next) => {
                    ptr = next.current;
                    new = Owned::new(Node {
                        val: unsafe { next.new.into_shared(&_guard).as_raw().read().val },
                        next: next.current.as_raw() as *mut _,
                    });
                }
            }
        }
    }

    pub fn pop(&self) -> Option<T> {
        let _g = epoch::pin();
        let mut top = self.top.load(Ordering::Acquire, &_g);
        let mut next = unsafe {
            Owned::from_raw(NonNull::new(top.as_raw().read().next)?.as_ptr() as *mut Node<T>)
        };
        loop {
            match self
                .top
                .compare_exchange(top, next, Ordering::AcqRel, Ordering::Acquire, &_g)
            {
                Ok(v) => break Some(unsafe { v.as_raw().read().val }),
                Err(new_top) => {
                    top = new_top.current;
                    next =
                        unsafe { Owned::from_raw(new_top.current.as_raw().read().next as *mut _) };
                }
            }
        }
    }
}

impl<T> const Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Node<T> {
    val: T,
    next: *mut CachePadded<Node<T>>,
}
