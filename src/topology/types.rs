//! Topology structural types.

use crate::law::{MemoryTier, NumaNodeId};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

/// NUMA node topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumaNode {
    /// Node identifier.
    pub id: NumaNodeId,
    /// Logical processors assigned to this node.
    pub processors: Box<[u32]>,
    /// Relative distance to other nodes.
    pub distances: Box<[u32]>,
    /// Primary memory tier for the node.
    pub memory_tier: MemoryTier,
}

/// Cache hierarchy level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLevel {
    /// Cache level.
    pub level: u32,
    /// Cache size in bytes.
    pub size_bytes: usize,
    /// Processors sharing this cache.
    pub shared_processors: Box<[u32]>,
}

/// Provider-supplied GPU device properties for [`crate::topology::GpuTopology::from_provider`].
///
/// A plain field struct (not a builder): every field is required, and the
/// provider reads them directly off the device API in one place. Fields the
/// API does not report are zero (capacity unknown), never fabricated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuDeviceProperties {
    /// Streaming-multiprocessor / compute-unit count (0 when unreported).
    pub compute_units: u32,
    /// Warp (NVIDIA) / wavefront (AMD) / subgroup width in lanes.
    pub warp_width: u32,
    /// Maximum resident threads per compute unit (0 when unreported).
    pub max_threads_per_unit: u32,
    /// 32-bit registers per compute unit (budgeted tier `Registers`;
    /// 0 when unreported).
    pub registers_per_unit: u32,
    /// Shared/local memory bytes per compute unit (budgeted tier
    /// `SharedMem`).
    pub shared_mem_per_unit_bytes: usize,
    /// Device L2 cache size in bytes (0 when unreported).
    pub l2_bytes: usize,
    /// Device global-memory tier (`Hbm`, `Gddr`, or `Device` when unknown).
    pub memory_tier: MemoryTier,
    /// Device global-memory capacity in bytes (0 when unreported).
    pub memory_bytes: u64,
}
