//! Topology version tokens.

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

    /// Returns the next epoch version.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}
