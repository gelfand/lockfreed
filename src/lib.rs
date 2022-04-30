#![allow(
    clippy::uninit_assumed_init,
    clippy::mem_replace_with_uninit,
    dead_code,
    soft_unstable
)]
#![feature(
    const_trait_impl,
    const_mut_refs,
    const_ptr_is_null,
    const_default_impls,
    allocator_api,
    maybe_uninit_uninit_array,
    const_replace,
    const_try,
    type_ascription,
    inline_const,
    test,
    bench_black_box,
    const_ptr_offset_from,
    const_ptr_write,
    const_ptr_read,
    const_intrinsic_copy
)]

pub mod stack;
