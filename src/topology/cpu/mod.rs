//! CPU topology snapshot and accessors.

mod cache;
mod detect;
mod tables;

use super::types::{CacheLevel, NumaNode};
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

pub(crate) use cache::default_cache_levels;
pub(crate) use tables::{build_adjacent_nodes, build_node_to_index};
#[cfg(any(test, feature = "std"))]
pub(crate) use tables::{build_default_distance_row, build_processor_to_node};
pub(in crate::topology) use tables::{LOCAL_DISTANCE, REMOTE_DISTANCE};

/// CPU topology snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Snapshot epoch.
    pub(crate) epoch: TopologyEpoch,
    pub(crate) numa_nodes: Box<[NumaNode]>,
    pub(crate) processor_to_node: Box<[NumaNodeId]>,
    pub(crate) node_to_index: Box<[usize]>,
    pub(crate) adjacent_nodes: Box<[NumaNodeId]>,
    pub(crate) logical_processors: usize,
    pub(crate) cache_levels: Box<[CacheLevel]>,
}

impl CpuTopology {
    /// Creates a single-node topology.
    #[must_use]
    pub fn single_node(logical_processors: usize) -> Self {
        let logical_processors = logical_processors.max(1);
        let processors: Box<[u32]> = (0..logical_processors as u32).collect();
        let node_id = NumaNodeId::ZERO;
        let numa_nodes: Box<[NumaNode]> = Box::new([NumaNode {
            id: node_id,
            processors,
            distances: Box::new([LOCAL_DISTANCE]),
            memory_tier: MemoryTier::Dram,
        }]);

        Self {
            epoch: TopologyEpoch::INITIAL,
            processor_to_node: vec![node_id; logical_processors].into_boxed_slice(),
            node_to_index: build_node_to_index(&numa_nodes),
            adjacent_nodes: build_adjacent_nodes(&numa_nodes),
            numa_nodes,
            logical_processors,
            cache_levels: default_cache_levels(logical_processors),
        }
    }

    /// Returns the snapshot epoch.
    #[must_use]
    pub const fn epoch(&self) -> TopologyEpoch {
        self.epoch
    }

    /// Returns the NUMA node table.
    #[must_use]
    pub fn numa_nodes(&self) -> &[NumaNode] {
        &self.numa_nodes
    }

    /// Returns the cache hierarchy table.
    #[must_use]
    pub fn cache_levels(&self) -> &[CacheLevel] {
        &self.cache_levels
    }

    /// Returns the logical processor count.
    #[must_use]
    pub const fn logical_processors(&self) -> usize {
        self.logical_processors
    }

    /// Returns the NUMA node for a processor.
    #[must_use]
    pub fn processor_to_numa_node(&self, processor: u32) -> Option<NumaNodeId> {
        self.processor_to_node
            .get(processor as usize)
            .copied()
            .filter(|&node_id| node_id != NumaNodeId::INVALID)
    }

    /// Iterates over known processor-to-node mappings.
    #[must_use = "iterators are lazy; consume the returned mapping iterator"]
    pub fn processor_node_pairs(&self) -> impl Iterator<Item = (u32, NumaNodeId)> + '_ {
        self.processor_to_node
            .iter()
            .enumerate()
            .filter_map(|(processor, &node)| {
                if node != NumaNodeId::INVALID {
                    Some((processor as u32, node))
                } else {
                    None
                }
            })
    }

    /// Returns node distance.
    #[must_use]
    pub fn distance(&self, from: NumaNodeId, to: NumaNodeId) -> u32 {
        match (self.node_index(from), self.node_index(to)) {
            (Some(from_index), Some(to_index)) => self
                .numa_nodes
                .get(from_index)
                .and_then(|node| {
                    let max_node_id = self.node_to_index.len().saturating_sub(1);
                    let idx = if node.distances.len() > max_node_id {
                        to.index()
                    } else {
                        to_index
                    };
                    node.distances.get(idx).copied()
                })
                .unwrap_or_else(|| tables::default_distance(from_index, to_index)),
            _ => {
                if from == to {
                    LOCAL_DISTANCE
                } else {
                    REMOTE_DISTANCE
                }
            }
        }
    }

    /// Returns the compact topology index for a NUMA node ID.
    #[must_use]
    pub fn node_index(&self, node_id: NumaNodeId) -> Option<usize> {
        self.node_to_index
            .get(node_id.index())
            .copied()
            .filter(|&index| index != usize::MAX)
    }

    /// Returns adjacent nodes sorted by distance.
    #[must_use]
    pub fn adjacent_nodes(&self, node_id: NumaNodeId) -> &[NumaNodeId] {
        if let Some(index) = self.node_index(node_id) {
            let node_count = self.numa_nodes.len();
            if node_count <= 1 {
                return &[];
            }
            let stride = node_count - 1;
            let start = index * stride;
            let end = start + stride;
            self.adjacent_nodes.get(start..end).unwrap_or(&[])
        } else {
            &[]
        }
    }
}

fn logical_processor_count() -> usize {
    #[cfg(feature = "std")]
    {
        std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
    }

    #[cfg(not(feature = "std"))]
    {
        1
    }
}
