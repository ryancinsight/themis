//! GPU topology query types.

use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

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
/// [`MemoryTier::is_host_allocatable`]). Every capacity accessor returns
/// `None` when the provider's API did not report it — unknowability is
/// type-level, never a sentinel zero.
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

    /// Streaming-multiprocessor / compute-unit count, when reported.
    #[must_use]
    #[inline]
    pub const fn compute_units(&self) -> Option<NonZeroU32> {
        self.properties.compute_units
    }

    /// Warp / wavefront / subgroup width in lanes, when reported.
    #[must_use]
    #[inline]
    pub const fn warp_width(&self) -> Option<NonZeroU32> {
        self.properties.warp_width
    }

    /// Maximum resident threads per compute unit, when reported.
    #[must_use]
    #[inline]
    pub const fn max_threads_per_unit(&self) -> Option<NonZeroU32> {
        self.properties.max_threads_per_unit
    }

    /// 32-bit registers per compute unit (budgeted `Registers` tier), when
    /// reported.
    #[must_use]
    #[inline]
    pub const fn registers_per_unit(&self) -> Option<NonZeroU32> {
        self.properties.registers_per_unit
    }

    /// Shared/local memory bytes per compute unit (budgeted `SharedMem`
    /// tier), when reported.
    #[must_use]
    #[inline]
    pub const fn shared_mem_per_unit_bytes(&self) -> Option<NonZeroUsize> {
        self.properties.shared_mem_per_unit_bytes
    }

    /// Device L2 cache size in bytes, when reported.
    #[must_use]
    #[inline]
    pub const fn l2_bytes(&self) -> Option<NonZeroUsize> {
        self.properties.l2_bytes
    }

    /// Device global-memory tier.
    #[must_use]
    #[inline]
    pub const fn memory_tier(&self) -> MemoryTier {
        self.properties.memory_tier
    }

    /// Device global-memory capacity in bytes, when reported.
    #[must_use]
    #[inline]
    pub const fn memory_bytes(&self) -> Option<NonZeroU64> {
        self.properties.memory_bytes
    }

    /// Total resident warps at theoretical full occupancy:
    /// `compute_units · max_threads_per_unit / warp_width`, when all three
    /// capacities are reported.
    #[must_use]
    #[inline]
    pub const fn max_resident_warps(&self) -> Option<u64> {
        match (
            self.properties.compute_units,
            self.properties.max_threads_per_unit,
            self.properties.warp_width,
        ) {
            (Some(units), Some(threads), Some(width)) => {
                Some((units.get() as u64) * (threads.get() as u64) / (width.get() as u64))
            }
            _ => None,
        }
    }
}
