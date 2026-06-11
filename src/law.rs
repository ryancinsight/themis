//! Placement law value types.

/// NUMA node identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumaNodeId(u32);

impl NumaNodeId {
    /// Node zero.
    pub const ZERO: Self = Self(0);

    /// Creates a NUMA node identifier.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw node value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns the node as an index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Maps this NUMA node into a fixed-size bucket table.
    ///
    /// # Panics
    ///
    /// Panics when `BUCKETS == 0`; zero buckets cannot represent a placement target.
    #[must_use]
    pub const fn bucket_index<const BUCKETS: usize>(self) -> NumaBucketIndex<BUCKETS> {
        NumaBucketIndex::new(self.index() % BUCKETS)
    }
}

/// NUMA bucket identity for a fixed-size placement table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumaBucketIndex<const BUCKETS: usize>(usize);

impl<const BUCKETS: usize> NumaBucketIndex<BUCKETS> {
    /// Creates a bucket index from an already-normalized raw index.
    ///
    /// # Panics
    ///
    /// Panics when `BUCKETS == 0`; zero buckets cannot represent a placement target.
    #[must_use]
    pub const fn new(raw: usize) -> Self {
        assert!(BUCKETS > 0, "NUMA bucket count must be non-zero");
        Self(raw % BUCKETS)
    }

    /// Returns the normalized bucket index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }

    /// Returns the next bucket after `offset` positions, wrapping inside the bucket table.
    #[must_use]
    pub const fn wrapping_add(self, offset: usize) -> Self {
        assert!(BUCKETS > 0, "NUMA bucket count must be non-zero");
        let offset = offset % BUCKETS;
        let remaining = BUCKETS - self.0;
        if offset < remaining {
            Self(self.0 + offset)
        } else {
            Self(offset - remaining)
        }
    }
}

/// Scheduler worker identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(u32);

impl WorkerId {
    /// Creates a worker identifier.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw worker value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns the worker as an index.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Coarse memory locality domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalityDomainId(u32);

impl LocalityDomainId {
    /// Creates a locality domain identifier.
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw domain value.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Topology version token.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopologyEpoch(u64);

impl TopologyEpoch {
    /// Initial topology epoch.
    pub const INITIAL: Self = Self(0);

    /// Creates a topology epoch.
    #[must_use]
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw epoch value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Memory tier classification.
///
/// Host-allocatable tiers (`Dram`, `Hbm`, `Gddr`, `HostPinned`, `Device`,
/// `Persistent`) are valid allocation targets for allocators such as
/// mnemosyne. The device-side tiers `Registers` and `SharedMem` are
/// **budgeted, non-host-allocatable** (atlas ADR 0002): GPU compilers assign
/// registers and kernels declare shared memory at launch, so these variants
/// exist purely as the typed vocabulary for capacity queries and kernel
/// resource budgets (occupancy planning) — never as allocation requests.
/// [`MemoryTier::is_host_allocatable`] encodes the distinction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// Standard host DRAM.
    #[default]
    Dram,
    /// High-bandwidth memory (host- or device-attached HBM stacks).
    Hbm,
    /// Device-attached GDDR memory (discrete-GPU global memory that is not
    /// HBM; distinct bandwidth/latency law from `Hbm`).
    Gddr,
    /// Page-locked (pinned) host memory for DMA staging transfers.
    HostPinned,
    /// Device-local memory of an unspecified technology.
    Device,
    /// Persistent memory.
    Persistent,
    /// GPU register file capacity (budgeted; compiler-assigned, never
    /// host-allocated).
    Registers,
    /// GPU shared/local memory per compute unit (budgeted; declared at
    /// kernel launch, never host-allocated).
    SharedMem,
}

impl MemoryTier {
    /// Returns true when the tier is a valid host-side allocation target.
    ///
    /// `Registers` and `SharedMem` return false: they are budget/capacity
    /// vocabulary for occupancy planning, not allocatable address space.
    #[must_use]
    #[inline]
    pub const fn is_host_allocatable(self) -> bool {
        !matches!(self, Self::Registers | Self::SharedMem)
    }
}

/// Placement preference supplied by allocation or scheduling callers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlacementHint {
    /// Use the caller's current locality.
    #[default]
    Current,
    /// Prefer the specified NUMA node.
    Numa(NumaNodeId),
    /// Prefer the specified locality domain.
    Domain(LocalityDomainId),
    /// Prefer the specified memory tier.
    Tier(MemoryTier),
    /// No locality preference.
    Any,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_ids_preserve_values() {
        assert_eq!(NumaNodeId::new(7).get(), 7);
        assert_eq!(NumaNodeId::new(7).index(), 7);
        assert_eq!(NumaNodeId::new(19).bucket_index::<16>().index(), 3);
        assert_eq!(
            NumaNodeId::new(19)
                .bucket_index::<16>()
                .wrapping_add(15)
                .index(),
            2
        );
        assert_eq!(WorkerId::new(3).get(), 3);
        assert_eq!(WorkerId::new(3).index(), 3);
        assert_eq!(LocalityDomainId::new(11).get(), 11);
        assert_eq!(TopologyEpoch::new(19).get(), 19);
    }

    #[test]
    fn default_placement_is_current_dram() {
        assert_eq!(PlacementHint::default(), PlacementHint::Current);
        assert_eq!(MemoryTier::default(), MemoryTier::Dram);
    }
}
