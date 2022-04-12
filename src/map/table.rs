#![allow(clippy::all)]
/// Derived from https://github.com/rust-lang/hashbrown
/// Licensed under either of Apache License, Version 2.0 or MIT license, at your option.
use std::{
    alloc::{Allocator, Global},
    marker::PhantomData,
    mem,
    ptr::NonNull,
    sync::atomic::AtomicUsize,
    usize,
};

use crossbeam::epoch::Atomic;

pub struct RawTable<T, A: Allocator + Clone = Global> {
    table: RawTableInner<A>,
    marker: PhantomData<T>,
}

pub struct RawTableInner<A> {
    bucket_mask: AtomicUsize,
    ctrl: Atomic<u8>,
    growth_late: AtomicUsize,
    items: AtomicUsize,
    alloc: A,
}

#[cfg(any(
    target_pointer_width = "64",
    target_arch = "aarch64",
    target_arch = "x86_64",
    target_arch = "wasm32",
))]
type GroupWord = u64;
#[cfg(all(
    target_pointer_width = "32",
    not(target_arch = "aarch64"),
    not(target_arch = "x86_64"),
    not(target_arch = "wasm32"),
))]

type GroupWord = u32;

#[derive(Copy, Clone)]
pub struct Group(GroupWord);

#[allow(clippy::use_self)]
impl Group {
    pub const WIDTH: usize = std::mem::size_of::<Self>();

    #[inline]
    pub const fn static_empty() -> &'static [u8; Group::WIDTH] {
        #[repr(C)]
        struct AlignedBytes {
            _align: [Group; 0],
            bytes: [u8; Group::WIDTH],
        }
        const ALIGNED_BYTES: AlignedBytes = AlignedBytes {
            _align: [],
            bytes: [0; Group::WIDTH],
        };
        &ALIGNED_BYTES.bytes
    }
}

impl<A> RawTableInner<A> {
    #[inline]
    pub fn new_in(alloc: A) -> Self {
        Self {
            ctrl: unsafe { Atomic::new((Group::static_empty() as *const _ as *mut u8).read()) },
            bucket_mask: AtomicUsize::new(0),
            growth_late: AtomicUsize::new(0),
            items: AtomicUsize::new(0),
            alloc,
        }
    }
}

impl<T> RawTable<T, Global> {
    #[inline]
    pub fn new() -> Self {
        Self {
            table: RawTableInner::new_in(Global),
            marker: PhantomData,
        }
    }
}

impl<T> Default for RawTable<T, Global> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct Bucket<T> {
    ptr: NonNull<T>,
}

const unsafe fn offset_from<T>(to: *const T, from: *const T) -> usize {
    to.offset_from(from) as usize
}
unsafe impl<T> Send for Bucket<T> {}

impl<T> const Clone for Bucket<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Bucket<T> {
    #[inline]
    pub const unsafe fn from_base_index(base: NonNull<T>, index: usize) -> Self {
        let ptr = if mem::size_of::<T>() == 0 {
            (index + 1) as *mut T
        } else {
            base.as_ptr().sub(index)
        };
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }

    #[inline]
    const unsafe fn to_base_index(&self, base: NonNull<T>) -> usize {
        if mem::size_of::<T>() == 0 {
            mem::transmute::<*mut T, usize>(self.ptr.as_ptr()) - 1
        } else {
            offset_from(base.as_ptr(), self.ptr.as_ptr())
        }
    }

    #[inline]
    pub const fn as_ptr(&self) -> *mut T {
        if mem::size_of::<T>() == 0 {
            mem::align_of::<T>() as *mut T
        } else {
            unsafe { self.ptr.as_ptr().sub(1) }
        }
    }

    pub const unsafe fn next_n(&self, offset: usize) -> Self {
        let ptr = if mem::size_of::<T>() == 0 {
            (mem::transmute::<*mut T, usize>(self.ptr.as_ptr()) + offset) as *mut T
        } else {
            self.ptr.as_ptr().sub(offset)
        };
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }

    #[inline]
    pub unsafe fn drop(&self) {
        self.ptr.as_ptr().drop_in_place();
    }
    #[inline]
    pub const unsafe fn read(&self) -> T {
        self.as_ptr().read()
    }
    #[inline]
    pub const unsafe fn write(&self, val: T) {
        self.as_ptr().write(val)
    }
    #[inline]
    pub const unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.as_ptr()
    }
    #[inline]
    pub const unsafe fn as_mut<'a>(&self) -> &'a mut T {
        &mut *self.as_ptr()
    }

    pub const unsafe fn copy_from_nonoverlapping(&self, other: &Self) {
        self.as_ptr().copy_from_nonoverlapping(other.as_ptr(), 1)
    }
}

#[cfg(test)]
#[cfg(not(miri))]
mod tests {
    use super::*;
    #[test]
    fn test_bucket_from_base_index() {
        unsafe {
            let base = NonNull::new_unchecked(0x1 as *mut u8);
            let bucket = Bucket::from_base_index(base, 0);
            assert_eq!(bucket.as_ptr(), base.as_ptr().sub(1));
            let bucket = Bucket::from_base_index(base, 1);
            assert_eq!(bucket.as_ptr(), base.as_ptr().sub(2));

            assert_eq!(Bucket::from_base_index(base, 0).to_base_index(base), 0);
            assert_eq!(Bucket::from_base_index(base, 1).to_base_index(base), 1);
            assert_eq!(Bucket::from_base_index(base, 2).to_base_index(base), 2);
        }
    }
}
