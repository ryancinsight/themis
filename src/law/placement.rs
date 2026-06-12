//! Placement preference vocabulary.

use super::{LocalityDomainId, MemoryTier, NumaNodeId};

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
