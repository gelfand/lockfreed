#![allow(
    clippy::uninit_assumed_init,
    clippy::mem_replace_with_uninit,
    dead_code
)]
#![feature(
    const_trait_impl,
    const_mut_refs,
    const_ptr_is_null,
    allocator_api,
    const_replace,
    const_try,
    type_ascription,
    inline_const,
    const_ptr_offset_from,
    const_ptr_write,
    const_ptr_read,
    const_intrinsic_copy
)]

pub mod map;
pub mod stack;
