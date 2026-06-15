//! Placement identity newtypes.

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
        NumaBucketIndex::new(self.index())
    }
}

/// NUMA bucket identity for a fixed-size placement table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumaBucketIndex<const BUCKETS: usize>(usize);

const fn assert_nonzero_buckets<const BUCKETS: usize>() {
    assert!(BUCKETS > 0, "NUMA bucket count must be non-zero");
}

impl<const BUCKETS: usize> NumaBucketIndex<BUCKETS> {
    /// Creates a bucket index from an already-normalized raw index.
    ///
    /// # Panics
    ///
    /// Panics when `BUCKETS == 0`; zero buckets cannot represent a placement target.
    #[must_use]
    pub const fn new(raw: usize) -> Self {
        assert_nonzero_buckets::<BUCKETS>();
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
        assert_nonzero_buckets::<BUCKETS>();
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
