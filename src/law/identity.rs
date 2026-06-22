//! Placement identity newtypes.

/// NUMA node identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumaNodeId(u32);

impl NumaNodeId {
    /// Node zero.
    pub const ZERO: Self = Self(0);

    /// Sentinel representing an invalid/unknown node.
    pub const INVALID: Self = Self(u32::MAX);

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

    /// Returns true when the node identifier is valid.
    #[must_use]
    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != Self::INVALID.0
    }

    /// Maps this NUMA node into a fixed-size bucket table.
    #[must_use]
    pub const fn bucket_index<const BUCKETS: usize>(self) -> NumaBucketIndex<BUCKETS> {
        NumaBucketIndex::new(self.index())
    }
}

/// NUMA bucket identity for a fixed-size placement table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumaBucketIndex<const BUCKETS: usize>(usize);

impl<const BUCKETS: usize> NumaBucketIndex<BUCKETS> {
    const ASSERT_NONZERO: () = {
        assert!(BUCKETS > 0, "NUMA bucket count must be non-zero");
    };

    /// Creates a bucket index from an already-normalized raw index.
    #[must_use]
    pub const fn new(raw: usize) -> Self {
        let () = Self::ASSERT_NONZERO;
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
        let () = Self::ASSERT_NONZERO;
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
    /// Sentinel representing an invalid/unknown worker.
    pub const INVALID: Self = Self(u32::MAX);

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

    /// Returns true when the worker identifier is valid.
    #[must_use]
    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != Self::INVALID.0
    }
}

/// Coarse memory locality domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalityDomainId(u32);

impl LocalityDomainId {
    /// Sentinel representing an invalid/unknown locality domain.
    pub const INVALID: Self = Self(u32::MAX);

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

    /// Returns true when the locality domain identifier is valid.
    #[must_use]
    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != Self::INVALID.0
    }
}
