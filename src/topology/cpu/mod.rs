//! CPU topology snapshot and accessors.

mod cache;
#[cfg(all(feature = "std", target_os = "linux"))]
mod cpulist;
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

#[cfg(feature = "std")]
pub(crate) use cache::detect_cache_levels;
#[cfg(all(feature = "std", target_os = "linux"))]
pub(crate) use cpulist::parse_cpu_list;
pub use tables::{build_adjacent_nodes, build_node_to_index};
#[cfg(any(test, feature = "std"))]
pub use tables::{build_default_distance_row, build_processor_to_node};
pub use tables::{LOCAL_DISTANCE, REMOTE_DISTANCE};

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
    pub(crate) cache_levels: Option<Box<[CacheLevel]>>,
}

impl CpuTopology {
    /// Creates a single-node topology.
    ///
    /// # Panics
    ///
    /// Panics if `logical_processors` exceeds `u32::MAX`; processor ids are
    /// `u32` throughout this crate, so a wider count has no representation.
    #[must_use]
    pub fn single_node(logical_processors: usize) -> Self {
        let logical_processors = logical_processors.max(1);
        let processor_count = u32::try_from(logical_processors)
            .expect("invariant: logical processor count must fit a u32 processor id");
        let processors: Box<[u32]> = (0..processor_count).collect();
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
            cache_levels: None,
        }
    }

    /// Construct a topology from primary fields for testing.
    ///
    /// `node_to_index` and `adjacent_nodes` are derived from `numa_nodes`.
    ///
    /// Gated on `feature = "testing"` (not just `cfg(test)`) because integration
    /// tests in `tests/` consume the lib as a regular dependency; `cfg(test)` only
    /// activates when the lib itself is the test target, not when it is depended on.
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn new_for_test(
        epoch: TopologyEpoch,
        numa_nodes: Box<[NumaNode]>,
        processor_to_node: Box<[NumaNodeId]>,
        logical_processors: usize,
        cache_levels: Option<Box<[CacheLevel]>>,
    ) -> Self {
        let node_to_index = build_node_to_index(&numa_nodes);
        let adjacent_nodes = build_adjacent_nodes(&numa_nodes);
        Self {
            epoch,
            numa_nodes,
            processor_to_node,
            node_to_index,
            adjacent_nodes,
            logical_processors,
            cache_levels,
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

    /// Returns the platform-reported cache hierarchy table.
    ///
    /// # Provenance
    ///
    /// `None` means the platform did not report a complete cache hierarchy.
    /// The single-node constructor never fabricates cache values. Linux reads
    /// cache-index records from sysfs, and Windows reads
    /// `GetLogicalProcessorInformationEx`; malformed or unavailable platform
    /// data remains typed absence. Consumers that tile on cache size must
    /// preserve that absence instead of substituting a machine-independent
    /// guess.
    #[must_use]
    pub fn cache_levels(&self) -> Option<&[CacheLevel]> {
        self.cache_levels.as_deref()
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
    ///
    /// # Panics
    ///
    /// The returned iterator panics if the processor table is longer than
    /// `u32::MAX`, which construction already caps at 32768 entries.
    #[must_use = "iterators are lazy; consume the returned mapping iterator"]
    pub fn processor_node_pairs(&self) -> impl Iterator<Item = (u32, NumaNodeId)> + '_ {
        self.processor_to_node
            .iter()
            .enumerate()
            .filter(|(_, &node)| node != NumaNodeId::INVALID)
            .map(|(processor, &node)| {
                // The processor table is capped at 32768 entries when built.
                let processor = u32::try_from(processor)
                    .expect("invariant: processor table length is capped at 32768");
                (processor, node)
            })
    }

    /// Returns node distance (ACPI SLIT convention: `10` = local, higher =
    /// farther).
    ///
    /// # Provenance
    ///
    /// Only the **Linux** backend reads real inter-node distances (from
    /// `/sys/devices/system/node/nodeN/distance`), falling back to the
    /// synthetic `10`/`20` matrix on read failure. The **Windows** backend has
    /// no distance API without `GetLogicalProcessorInformationEx` relative-
    /// distance parsing, so it always returns the synthetic `10` (local) /
    /// `20` (remote) — uniform regardless of true inter-node latency. Consumers
    /// that weight placement by distance must treat a Windows result as a
    /// two-tier local/remote hint, not a measured latency.
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
        std::thread::available_parallelism().map_or(1, usize::from)
    }

    #[cfg(not(feature = "std"))]
    {
        1
    }
}
