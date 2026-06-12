//! GPU topology query types.

use super::types::GpuDeviceProperties;
use crate::law::{MemoryTier, TopologyEpoch};

/// GPU device topology snapshot (atlas ADR 0002).
///
/// Provider-fed: themis stays stateless law, so there is no `detect()` here —
/// device backends (hephaestus) construct this from wgpu adapter limits or
/// CUDA device attributes via [`GpuTopology::from_provider`]. Consumers:
/// moirai's occupancy planner (warp-aware launch shaping) and mnemosyne's
/// kernel resource budgets read these capacities; the `Registers`/`SharedMem`
/// figures are budget vocabulary, never host-allocatable (see
/// [`MemoryTier::is_host_allocatable`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTopology {
    epoch: TopologyEpoch,
    properties: GpuDeviceProperties,
}

impl GpuTopology {
    /// Construct a snapshot from provider-reported device properties.
    #[must_use]
    pub const fn from_provider(properties: GpuDeviceProperties) -> Self {
        Self {
            epoch: TopologyEpoch::INITIAL,
            properties,
        }
    }

    /// Snapshot epoch.
    #[must_use]
    #[inline]
    pub const fn epoch(&self) -> TopologyEpoch {
        self.epoch
    }

    /// Streaming-multiprocessor / compute-unit count.
    #[must_use]
    #[inline]
    pub const fn compute_units(&self) -> u32 {
        self.properties.compute_units
    }

    /// Warp / wavefront / subgroup width in lanes.
    #[must_use]
    #[inline]
    pub const fn warp_width(&self) -> u32 {
        self.properties.warp_width
    }

    /// Maximum resident threads per compute unit.
    #[must_use]
    #[inline]
    pub const fn max_threads_per_unit(&self) -> u32 {
        self.properties.max_threads_per_unit
    }

    /// 32-bit registers per compute unit (budgeted `Registers` tier).
    #[must_use]
    #[inline]
    pub const fn registers_per_unit(&self) -> u32 {
        self.properties.registers_per_unit
    }

    /// Shared/local memory bytes per compute unit (budgeted `SharedMem` tier).
    #[must_use]
    #[inline]
    pub const fn shared_mem_per_unit_bytes(&self) -> usize {
        self.properties.shared_mem_per_unit_bytes
    }

    /// Device L2 cache size in bytes (0 when unreported).
    #[must_use]
    #[inline]
    pub const fn l2_bytes(&self) -> usize {
        self.properties.l2_bytes
    }

    /// Device global-memory tier.
    #[must_use]
    #[inline]
    pub const fn memory_tier(&self) -> MemoryTier {
        self.properties.memory_tier
    }

    /// Device global-memory capacity in bytes.
    #[must_use]
    #[inline]
    pub const fn memory_bytes(&self) -> u64 {
        self.properties.memory_bytes
    }

    /// Total resident warps at theoretical full occupancy:
    /// `compute_units · max_threads_per_unit / warp_width`. Returns 0 for a
    /// zero warp width rather than dividing by zero.
    #[must_use]
    #[inline]
    pub const fn max_resident_warps(&self) -> u64 {
        if self.properties.warp_width == 0 {
            return 0;
        }
        (self.properties.compute_units as u64) * (self.properties.max_threads_per_unit as u64)
            / (self.properties.warp_width as u64)
    }
}
