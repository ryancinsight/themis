//! GPU topology structural types.

use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use crate::law::MemoryTier;

/// Provider-supplied GPU device properties for [`crate::GpuTopology::from_provider`].
///
/// A plain field struct (not a builder): every field is required, and the
/// provider reads them directly off the device API in one place. A capacity
/// the API does not report is `None` — unknowability lives in the type, so
/// no consumer can divide by or partition over a sentinel zero, and a
/// fabricated value is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuDeviceProperties {
    /// Streaming-multiprocessor / compute-unit count (`None` when
    /// unreported).
    pub compute_units: Option<NonZeroU32>,
    /// Warp (NVIDIA) / wavefront (AMD) / subgroup width in lanes (`None`
    /// when unreported).
    pub warp_width: Option<NonZeroU32>,
    /// Maximum resident threads per compute unit (`None` when unreported).
    pub max_threads_per_unit: Option<NonZeroU32>,
    /// 32-bit registers per compute unit (budgeted tier `Registers`;
    /// `None` when unreported).
    pub registers_per_unit: Option<NonZeroU32>,
    /// Shared/local memory bytes per compute unit (budgeted tier
    /// `SharedMem`; `None` when unreported).
    pub shared_mem_per_unit_bytes: Option<NonZeroUsize>,
    /// Device L2 cache size in bytes (`None` when unreported).
    pub l2_bytes: Option<NonZeroUsize>,
    /// Device global-memory tier (`Hbm`, `Gddr`, or `Device` when unknown).
    pub memory_tier: MemoryTier,
    /// Device global-memory capacity in bytes (`None` when unreported).
    pub memory_bytes: Option<NonZeroU64>,
}
