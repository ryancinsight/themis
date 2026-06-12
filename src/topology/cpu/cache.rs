//! Conservative CPU cache hierarchy defaults.

use super::CacheLevel;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub(in crate::topology) fn default_cache_levels(logical_processors: usize) -> Box<[CacheLevel]> {
    let processors: Vec<u32> = (0..logical_processors.max(1) as u32).collect();
    vec![
        CacheLevel {
            level: 1,
            size_bytes: 32 * 1024,
            shared_processors: Box::default(),
        },
        CacheLevel {
            level: 2,
            size_bytes: 256 * 1024,
            shared_processors: Box::default(),
        },
        CacheLevel {
            level: 3,
            size_bytes: 8 * 1024 * 1024,
            shared_processors: processors.into_boxed_slice(),
        },
    ]
    .into_boxed_slice()
}
