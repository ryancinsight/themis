//! CPU cache hierarchy discovery.

use super::CacheLevel;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(all(feature = "std", target_os = "linux"))]
mod linux;
#[cfg(all(feature = "std", windows))]
mod windows;

#[cfg(feature = "std")]
pub(crate) fn detect_cache_levels(_logical_processors: usize) -> Option<Box<[CacheLevel]>> {
    #[cfg(target_os = "linux")]
    {
        linux::detect()
    }

    #[cfg(windows)]
    {
        windows::detect()
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    {
        None
    }
}

#[cfg(not(feature = "std"))]
pub(crate) const fn detect_cache_levels(_logical_processors: usize) -> Option<Box<[CacheLevel]>> {
    None
}
