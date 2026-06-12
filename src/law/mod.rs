//! Placement law value types.

mod epoch;
mod identity;
mod memory;
mod placement;

#[cfg(test)]
mod tests;

pub use epoch::TopologyEpoch;
pub use identity::{LocalityDomainId, NumaBucketIndex, NumaNodeId, WorkerId};
pub use memory::MemoryTier;
pub use placement::PlacementHint;
