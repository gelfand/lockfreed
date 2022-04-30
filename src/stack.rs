extern crate crossbeam;
use core::{ptr, sync::atomic::AtomicUsize, sync::atomic::Ordering};
use crossbeam::{
    epoch::{self, Atomic, Owned},
    utils::CachePadded,
};

#[derive(Debug)]
pub struct Stack<T> {
    top: Atomic<CachePadded<Node<T>>>,
    size: AtomicUsize,
}

impl<T> Stack<T> {
    /// Create a new empty `Stack<T>`.
    ///
    /// # Examples
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let s = Stack::<()>::new();
    ///
    /// assert!(s.is_empty());
    /// ```
    ///
    #[inline(always)]
    #[cfg(feature = "nightly")]
    pub const fn new() -> Self {
        Self {
            top: Atomic::null(),
            size: Default::default(),
        }
    }

    #[inline(always)]
    #[cfg(not(feature = "nightly"))]
    pub fn new() -> Self {
        Self {
            top: Atomic::null(),
            size: Default::default(),
        }
    }

    /// Pushes a value onto a stack.
    ///
    /// # Examples
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let mut s = Stack::<i32>::new();
    ///
    /// s.push(1);
    /// assert_eq!(s.pop(), Some(1));
    /// ```
    #[inline(always)]
    pub fn push(&self, val: T) {
        self.size.fetch_add(1, Ordering::Relaxed);

        let mut n = Owned::new(CachePadded::new(Node {
            val,
            next: Atomic::null(),
        }));

        let g = epoch::pin();

        loop {
            let ptr = self.top.load(Ordering::Acquire, &g);
            n.next.store(ptr, Ordering::Release);
            match self
                .top
                .compare_exchange(ptr, n, Ordering::Release, Ordering::Relaxed, &g)
            {
                Ok(_) => break,
                Err(e) => n = e.new,
            }
        }
    }

    /// Pops the element from the top of the stack.
    ///
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let s = Stack::new();
    ///
    /// s.push(1);
    ///
    /// assert_eq!(s.pop(), Some(1));
    /// assert_eq!(s.pop(), None);
    /// ```
    #[inline(always)]
    pub fn pop(&self) -> Option<T> {
        let g = epoch::pin();
        loop {
            let ptr = self.top.load(Ordering::Acquire, &g);
            match unsafe { ptr.as_ref() } {
                Some(head) => {
                    let next = head.next.load(Ordering::Relaxed, &g);
                    if self
                        .top
                        .compare_exchange(ptr, next, Ordering::Release, Ordering::Relaxed, &g)
                        .is_ok()
                    {
                        self.size.fetch_sub(1, Ordering::Relaxed);
                        unsafe {
                            break Some(ptr::read(&(*head).val));
                        }
                    }
                }
                None => return None,
            }
        }
    }

    /// Returns the number of elements in the stack.
    ///
    /// # Examples
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let s = Stack::new();
    ///
    /// assert_eq!(s.len(), 0);
    ///
    /// s.push(1);
    ///
    /// assert_eq!(s.len(), 1);
    /// ```
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Returns `true` if the stack contains no elements.
    ///
    /// # Examples
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let s = Stack::new();
    ///
    /// assert!(s.is_empty());
    ///
    /// s.push(1);
    ///
    /// assert!(!s.is_empty());
    /// ```
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extends the stack with the given iterable. Acts just like
    /// [`Extend::extend`](https://doc.rust-lang.org/std/iter/trait.Extend.html#method.extend)`
    /// but doesn't require mutability.
    ///
    /// # Examples
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let mut s = Stack::new();
    ///
    /// s.extend(1..=3);
    ///
    /// assert_eq!(s.pop(), Some(3));
    /// assert_eq!(s.pop(), Some(2));
    /// assert_eq!(s.pop(), Some(1));
    /// assert_eq!(s.pop(), None);
    /// ```
    #[inline]
    pub fn extend<I>(&self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for val in iter {
            self.push(val);
        }
    }

    /// Clears the stack, removing all elements.
    ///
    /// # Examples
    /// ```
    /// use lockfreed::stack::Stack;
    ///
    /// let mut s = Stack::new();
    ///
    /// s.push(1);
    /// s.push(2);
    ///
    /// assert_eq!(s.len(), 2);
    ///
    /// s.clear();
    ///
    /// assert_eq!(s.len(), 0);
    /// ```
    #[inline(always)]
    pub fn clear(&self) {
        while self.pop().is_some() {}
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Stack<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[derive(Debug)]
struct Node<T> {
    val: T,
    next: Atomic<CachePadded<Node<T>>>,
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use super::*;
    use alloc::sync::Arc;
    use alloc::vec::Vec;

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

    #[cfg_attr(feature = "std", test)]
    fn test_multithread_stack() {
        extern crate std;

        let s = Arc::new(Stack::new());

        let mut handles = Vec::with_capacity(8);
        for _ in 0..8 {
            handles.push(std::thread::spawn({
                let s = s.clone();
                move || {
                    for i in 0..100 {
                        s.push(i * 2);
                        s.push(i * 4);
                        s.pop();
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(s.len(), 800);
        let mut left = 0;
        while let Some(_) = s.pop() {
            left += 1;
        }
        assert_eq!(left, 800);
        assert_eq!(s.len(), 0);
    }

    #[cfg_attr(feature = "std", test)]
    fn no_data_corruption() {
        extern crate std;

        const NTHREAD: usize = 20;
        const NITER: usize = 800;
        const NMOD: usize = 55;

        let stack = Arc::new(Stack::new());
        let mut handles = Vec::with_capacity(NTHREAD);

        for i in 0..NTHREAD {
            let stack = stack.clone();
            handles.push(std::thread::spawn(move || {
                for j in 0..NITER {
                    let val = (i * NITER) + j;
                    stack.push(val);
                    if (val + 1) % NMOD == 0 {
                        if let Some(val) = stack.pop() {
                            assert!(val < NITER * NTHREAD);
                        }
                    }
                }
            }));
        }

        for handle in handles {
            handle.join().expect("thread failed");
        }

        let expected = NITER * NTHREAD - NITER * NTHREAD / NMOD;
        let mut res = 0;
        while let Some(val) = stack.pop() {
            assert!(val < NITER * NTHREAD);
            res += 1;
        }

        assert_eq!(res, expected);
    }
}
