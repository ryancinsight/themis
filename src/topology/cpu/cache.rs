//! CPU cache hierarchy discovery.

#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
use super::super::types::CacheLevel;

#[cfg(all(feature = "std", target_os = "linux"))]
mod linux;
#[cfg(all(feature = "std", windows))]
mod windows;

#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
pub(crate) fn detect_cache_levels(_logical_processors: usize) -> Option<Box<[CacheLevel]>> {
    #[cfg(target_os = "linux")]
    {
        linux::detect()
    }

    #[cfg(windows)]
    {
        windows::detect()
    }
}
