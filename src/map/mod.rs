use crossbeam::epoch::{self, Collector, Owned};
use crossbeam_skiplist::SkipList;
use std::ptr::{null_mut, NonNull};

pub struct Bucket {
    hash: u64,
    /// Key is is an (entry_ptr as usize) << hash.
    /// Values is just an (entry_ptr as usize).
    list: SkipList<usize, usize>,
}

pub struct Entry<K, V> {
    pair: NonNull<(K, V)>,
    next: Owned<SkipList<K, V>>,
}

static _NON_NULL: u8 = 1;

#[inline(always)]
pub fn non_zero_null<T>() -> NonNull<T> {
    NonNull::from(&_NON_NULL).cast()
}

impl<K, V> Entry<K, V> {
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn root(next: *mut SkipList<K, V>) -> Self {
        Self {
            // Use this dangling pointer to mark an entry as the "sentinel"
            // "root" entry.
            pair: non_zero_null(),
            next: unsafe { Owned::from_raw(next) },
        }
    }
}

unsafe impl<K, V> Send for Entry<K, V> {}
unsafe impl<K, V> Sync for Entry<K, V> {}

impl Bucket {
    #[inline]
    pub fn new<K, V>(hash: u64, pair: NonNull<(K, V)>) -> Self {
        // We create a bucket with a single entry.

        // First we create an entry for the pair whose next node is null.
        let mut entry = Entry {
            pair,
            next: unsafe { Owned::from_raw(null_mut()) },
        };
        let list = SkipList::new(Collector::default());

        let _g = epoch::pin();
        let entry_ptr = &mut entry as *mut Entry<K, V>;
        list.insert((entry_ptr as usize) << hash, entry_ptr as usize, &_g);

        Self { hash, list }
    }
}

pub struct HashMap<K, V> {
    v: SkipList<K, V>,
}
