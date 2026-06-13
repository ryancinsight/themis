//! Platform CPU topology detection.

#[cfg(all(feature = "std", target_os = "linux"))]
mod linux;
#[cfg(all(feature = "std", windows))]
mod windows;

#[cfg(not(any(
    all(feature = "std", target_os = "linux"),
    all(feature = "std", windows)
)))]
use super::logical_processor_count;
use super::CpuTopology;

impl CpuTopology {
    /// Detects the CPU topology from the platform.
    #[must_use]
    pub fn detect() -> Option<Self> {
        #[cfg(all(feature = "std", target_os = "linux"))]
        {
            linux::detect()
        }

        #[cfg(all(feature = "std", windows))]
        {
            windows::detect()
        }

        #[cfg(not(any(
            all(feature = "std", target_os = "linux"),
            all(feature = "std", windows)
        )))]
        {
            Some(Self::single_node(logical_processor_count()))
        }
    }
}
