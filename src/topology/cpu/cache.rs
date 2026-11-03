//! CPU cache hierarchy discovery.

#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
use super::super::types::CacheLevel;

#[cfg(all(feature = "std", any(target_os = "linux", test)))]
mod linux;
#[cfg(all(feature = "std", windows))]
mod windows;

#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
pub(crate) fn detect_cache_levels(_logical_processors: usize) -> Option<Box<[CacheLevel]>> {
    #[cfg(target_os = "linux")]
    {
        linux::detect(std::path::Path::new("/sys/devices/system/cpu"))
    }

    #[cfg(windows)]
    {
        windows::detect()
    }
}
