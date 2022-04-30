#![no_std]
#![allow(dead_code)]
#![cfg_attr(feature = "nightly", feature(const_trait_impl, const_default_impls))]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod stack;
