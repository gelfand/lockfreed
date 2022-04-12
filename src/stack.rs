extern crate crossbeam;
#[allow(deprecated)]
use crossbeam::{
    epoch::{self, Atomic, CompareAndSetOrdering, Owned},
    utils::CachePadded,
};
use std::{
    ptr::{self},
    sync::atomic::Ordering,
};

#[derive(Debug)]
pub struct Stack<T> {
    top: Atomic<CachePadded<Node<T>>>,
}

impl<T> Stack<T> {
    pub const fn new() -> Self {
        Self {
            top: Atomic::null(),
        }
    }
    pub fn push(&self, val: T) {
        let mut n = Owned::new(CachePadded::new(Node {
            val,
            next: Atomic::null(),
        }));
        let g = epoch::pin();

        loop {
            let ptr = self.top.load(Ordering::Relaxed, &g);
            n.next.store(ptr, Ordering::Relaxed);
            #[allow(deprecated)]
            match self.top.compare_exchange(
                ptr,
                n,
                Ordering::Release.success(),
                Ordering::Release.failure(),
                &g,
            ) {
                Ok(_) => break,
                Err(e) => n = e.new,
            }
        }
    }

    #[allow(deprecated)]
    pub fn pop(&self) -> Option<T> {
        let g = epoch::pin();
        loop {
            let ptr = self.top.load(Ordering::Acquire, &g);
            match unsafe { ptr.as_ref() } {
                Some(head) => {
                    let next = head.next.load(Ordering::Relaxed, &g);
                    if self
                        .top
                        .compare_exchange(
                            ptr,
                            next,
                            Ordering::Release.success(),
                            Ordering::Release.failure(),
                            &g,
                        )
                        .is_ok()
                    {
                        unsafe {
                            g.defer_unchecked(move || ptr.into_owned());
                            return Some(ptr::read(&(*head).val));
                        }
                    }
                }
                None => return None,
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
    next: Atomic<CachePadded<Node<T>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_stack() {
        let s = Stack::new();
        s.push(1);
        s.push(2);
        s.push(3);
        assert_eq!(s.pop(), Some(3));
        assert_eq!(s.pop(), Some(2));
        assert_eq!(s.pop(), Some(1));
        assert_eq!(s.pop(), None);
    }
}
