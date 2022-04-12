use std::{cell::UnsafeCell, sync::atomic::Ordering};

use crossbeam::{
    epoch::{self, Atomic, Owned, Shared},
    utils::CachePadded,
};

/// Queue FIFO lockfree.
pub struct Queue<T> {
    head: Atomic<CachePadded<Node<T>>>,
    tail: Atomic<CachePadded<Node<T>>>,
}

pub struct Node<T> {
    val: *mut T,
    next: Atomic<CachePadded<Node<T>>>,
}

impl<T> Node<T> {
    pub fn new(val: T) -> Self {
        Self {
            val: Box::into_raw(Box::new(val)),
            next: Atomic::null(),
        }
    }
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
            tail: Atomic::null(),
        }
    }

    pub fn push(&self, val: T) {
        let guard = epoch::pin();
        let ptr = Owned::new(Node::new(val)).into_shared(&guard).as_raw() as *mut _;
        let mut tail: epoch::Shared<CachePadded<Node<T>>> =
            self.tail
                .swap(unsafe { Owned::from_raw(ptr) }, Ordering::AcqRel, &guard);
        if let Some(tail) = unsafe { tail.as_ref() } {
            tail.next
                .store(unsafe { Owned::from_raw(ptr) }, Ordering::Release);
        }
    }

    pub fn pop(&self) -> Option<T> {
        let guard = epoch::pin();
        let mut head: epoch::Shared<CachePadded<Node<T>>> =
            self.head.load(Ordering::Acquire, &guard);
        loop {
            if let Some(h) = unsafe { head.as_ref() } {
                let next = h.next.load(Ordering::Acquire, &guard);
                if unsafe { next.as_ref() }.is_some() {
                    head = next;
                } else {
                    let next = h.next.swap(Shared::null(), Ordering::AcqRel, &guard);
                    if next.is_null() {
                        self.head.store(Shared::null(), Ordering::Release);
                        unsafe {
                            return Some(h.val.read());
                        }
                    } else {
                        head = next;
                    }
                }
            } else {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop() {
        let q = Queue::new();
        q.push(1);
        q.push(2);
        q.push(3);
        q.push(4);
        q.push(5);

        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), Some(4));
        assert_eq!(q.pop(), Some(5));
        assert_eq!(q.pop(), None);
    }
}

impl<T> Default for Queue<T> {
    fn default() -> Self {
        Self::new()
    }
}
