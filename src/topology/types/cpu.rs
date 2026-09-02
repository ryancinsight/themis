//! CPU topology structural types.

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

/// Relative performance rank of a logical processor on a hybrid CPU.
///
/// This is an **ordinal**, not a boolean. Rank `0` is the least performant
/// class the platform reports and higher ranks are more performant, matching
/// the direction of the Windows `PROCESSOR_RELATIONSHIP::EfficiencyClass` byte
/// and of the Linux `cpu_capacity` value. A part with performance, efficient,
/// and low-power-efficient tiers is three ranks, not a two-way split.
///
/// Ranks are **dense**: a topology reporting `n` classes uses exactly the ranks
/// `0..n`, whatever sparse or arbitrary values the platform used underneath.
/// Ranks are therefore comparable only within one [`CpuTopology`] snapshot;
/// they are not a cross-machine performance scale.
///
/// [`CpuTopology`]: crate::CpuTopology
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EfficiencyClass(u8);

impl EfficiencyClass {
    /// The least performant class of any topology that reports classes.
    ///
    /// A homogeneous host reports this rank for every processor.
    pub const LOWEST: Self = Self(0);

    /// Constructs a class from its dense rank.
    #[must_use]
    pub const fn new(rank: u8) -> Self {
        Self(rank)
    }

    /// Returns the dense rank, higher meaning more performant.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0
    }
}

/// Identity of a physical core within one [`CpuTopology`] snapshot.
///
/// Logical processors that are SMT siblings share one `CoreId`. Ids are dense:
/// a topology with `n` physical cores uses exactly `0..n`, assigned in
/// ascending order of each core's lowest logical processor, so they are
/// comparable only within one snapshot and never a hardware core number.
///
/// Two processors with different ids are interchangeable for placement in the
/// sense this axis models: they do not share an execution core. Two with the
/// same id do, and a worker pool that treats them as two cores oversubscribes.
///
/// [`CpuTopology`]: crate::CpuTopology
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CoreId(u32);

impl CoreId {
    /// Constructs a core id from its dense index.
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the dense index.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Cache hierarchy level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLevel {
    /// Cache level.
    pub level: u32,
    /// Cache size in bytes.
    pub size_bytes: usize,
    /// Cache-line size in bytes when the provider reports it.
    pub line_bytes: Option<usize>,
    /// Processors sharing this cache.
    pub shared_processors: Box<[u32]>,
}
