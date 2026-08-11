//! CPU cache hierarchy discovery.

#[cfg(feature = "std")]
use super::CacheLevel;

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
