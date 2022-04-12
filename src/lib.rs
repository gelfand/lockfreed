#![allow(
    clippy::uninit_assumed_init,
    clippy::mem_replace_with_uninit,
    dead_code
)]
#![feature(
    const_trait_impl,
    const_mut_refs,
    const_ptr_is_null,
    const_replace,
    const_try,
    inline_const
)]

pub mod stack;
