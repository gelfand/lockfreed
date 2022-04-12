pub mod table;
use hashbrown::{hash_map::DefaultHashBuilder, raw::RawTable};
use std::alloc::Allocator;
use std::alloc::Global;

pub struct HashMap<K, V, S = DefaultHashBuilder, A: Allocator + Clone = Global> {
    pub(crate) hash_builder: S,
    pub(crate) table: RawTable<(K, V), A>,
}

impl<K: Clone, V: Clone, S: Clone, A: Allocator + Clone> Clone for HashMap<K, V, S, A> {
    fn clone(&self) -> Self {
        HashMap {
            hash_builder: self.hash_builder.clone(),
            table: self.table.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.table.clone_from(&source.table);

        // Update hash_builder only if we successfully cloned all elements.
        self.hash_builder.clone_from(&source.hash_builder);
    }
}

impl<K, V, S> HashMap<K, V, S> {
    #[inline]
    pub const fn with_hasher(hash_builder: S) -> Self {
        HashMap {
            hash_builder,
            table: RawTable::new(),
        }
    }

    #[inline]
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        HashMap {
            hash_builder,
            table: RawTable::with_capacity(capacity),
        }
    }
}
