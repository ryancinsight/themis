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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// Standard host DRAM.
    #[default]
    Dram,
    /// High-bandwidth memory.
    Hbm,
    /// Device-local memory.
    Device,
    /// Persistent memory.
    Persistent,
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
