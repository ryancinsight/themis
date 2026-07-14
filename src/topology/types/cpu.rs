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
