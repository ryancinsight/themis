//! CPU topology detection and snapshot queries.

use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};
use super::types::{CacheLevel, NumaNode};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// CPU topology snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Snapshot epoch.
    pub(super) epoch: TopologyEpoch,
    pub(super) numa_nodes: Box<[NumaNode]>,
    pub(super) processor_to_node: Box<[Option<NumaNodeId>]>,
    pub(super) node_to_index: Box<[Option<usize>]>,
    pub(super) adjacent_nodes: Box<[Box<[NumaNodeId]>]>,
    pub(super) logical_processors: usize,
    pub(super) cache_levels: Box<[CacheLevel]>,
}

impl CpuTopology {
    /// Detects the CPU topology from the platform.
    #[must_use]
    pub fn detect() -> Option<Self> {
        #[cfg(all(feature = "std", target_os = "linux"))]
        {
            Self::detect_linux()
        }

        #[cfg(all(feature = "std", windows))]
        {
            Self::detect_windows()
        }

        #[cfg(not(any(
            all(feature = "std", target_os = "linux"),
            all(feature = "std", windows)
        )))]
        {
            Some(Self::single_node(logical_processor_count()))
        }
    }

    /// Creates a single-node topology.
    #[must_use]
    pub fn single_node(logical_processors: usize) -> Self {
        let logical_processors = logical_processors.max(1);
        let processors: Vec<u32> = (0..logical_processors as u32).collect();
        let node_id = NumaNodeId::ZERO;
        let processor_node_pairs: Vec<(u32, NumaNodeId)> = processors
            .iter()
            .map(|processor| (*processor, node_id))
            .collect();
        let numa_nodes = vec![NumaNode {
            id: node_id,
            processors: processors.into_boxed_slice(),
            distances: vec![10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        }];

        Self {
            epoch: TopologyEpoch::INITIAL,
            processor_to_node: build_processor_to_node(logical_processors, &processor_node_pairs),
            node_to_index: build_node_to_index(&numa_nodes),
            adjacent_nodes: build_adjacent_nodes(&numa_nodes),
            numa_nodes: numa_nodes.into_boxed_slice(),
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
            .flatten()
    }

    /// Iterates over known processor-to-node mappings.
    pub fn processor_node_pairs(&self) -> impl Iterator<Item = (u32, NumaNodeId)> + '_ {
        self.processor_to_node
            .iter()
            .enumerate()
            .filter_map(|(processor, node)| Some((processor as u32, (*node)?)))
    }

    /// Returns node distance.
    #[must_use]
    pub fn distance(&self, from: NumaNodeId, to: NumaNodeId) -> u32 {
        match (self.node_index(from), self.node_index(to)) {
            (Some(from_index), Some(to_index)) => self
                .numa_nodes
                .get(from_index)
                .and_then(|node| node.distances.get(to_index).copied())
                .unwrap_or(if from == to { 10 } else { 20 }),
            _ => {
                if from == to {
                    10
                } else {
                    20
                }
            }
        }
    }

    /// Returns the compact topology index for a NUMA node ID.
    #[must_use]
    pub fn node_index(&self, node_id: NumaNodeId) -> Option<usize> {
        self.node_to_index.get(node_id.index()).copied().flatten()
    }

    /// Returns adjacent nodes sorted by distance.
    #[must_use]
    pub fn adjacent_nodes(&self, node_id: NumaNodeId) -> &[NumaNodeId] {
        self.node_index(node_id)
            .and_then(|index| self.adjacent_nodes.get(index))
            .map_or(&[], |nodes| nodes)
    }

    #[cfg(all(feature = "std", target_os = "linux"))]
    fn detect_linux() -> Option<Self> {
        use std::fs;

        let nodes_path = "/sys/devices/system/node/";
        let node_entries = fs::read_dir(nodes_path).ok();
        let mut node_ids: Vec<u32> = node_entries
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| name.strip_prefix("node")?.parse::<u32>().ok())
            .collect();
        node_ids.sort_unstable();

        if node_ids.is_empty() {
            return Some(Self::single_node(logical_processor_count()));
        }

        let mut numa_nodes = Vec::with_capacity(node_ids.len());
        let mut processor_node_pairs = Vec::new();

        for node_id_raw in &node_ids {
            let node_id = NumaNodeId::new(*node_id_raw);
            let cpulist_path = format!("{nodes_path}/node{node_id_raw}/cpulist");
            let processors = fs::read_to_string(cpulist_path)
                .map(|value| parse_cpu_list(&value))
                .unwrap_or_default();

            for processor in &processors {
                processor_node_pairs.push((*processor, node_id));
            }

            let distance_path = format!("{nodes_path}/node{node_id_raw}/distance");
            let distances = fs::read_to_string(distance_path)
                .map(|value| {
                    value
                        .split_whitespace()
                        .filter_map(|part| part.parse::<u32>().ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|_| vec![10; node_ids.len()]);

            numa_nodes.push(NumaNode {
                id: node_id,
                processors: processors.into_boxed_slice(),
                distances: distances.into_boxed_slice(),
                memory_tier: MemoryTier::Dram,
            });
        }

        let logical_processors = logical_processor_count();
        let processor_to_node = build_processor_to_node(logical_processors, &processor_node_pairs);
        let node_to_index = build_node_to_index(&numa_nodes);
        let adjacent_nodes = build_adjacent_nodes(&numa_nodes);
        Some(Self {
            epoch: TopologyEpoch::INITIAL,
            numa_nodes: numa_nodes.into_boxed_slice(),
            node_to_index,
            processor_to_node,
            adjacent_nodes,
            logical_processors,
            cache_levels: default_cache_levels(logical_processors),
        })
    }

    #[cfg(all(feature = "std", windows))]
    fn detect_windows() -> Option<Self> {
        extern "system" {
            fn GetNumaHighestNodeNumber(highest_node_number: *mut u32) -> i32;
            fn GetNumaNodeProcessorMask(node: u8, processor_mask: *mut u64) -> i32;
        }

        let mut highest_node = 0u32;
        // SAFETY: The API writes one `u32` through a valid output pointer.
        if unsafe { GetNumaHighestNodeNumber(&mut highest_node) } == 0 {
            return Some(Self::single_node(logical_processor_count()));
        }

        let node_count = highest_node.saturating_add(1) as usize;
        let mut numa_nodes = Vec::with_capacity(node_count);
        let mut processor_node_pairs = Vec::new();
        let mut logical_processors = 0usize;

        for raw_node in 0..=highest_node {
            let mut mask = 0u64;
            // SAFETY: The API writes one processor mask through a valid pointer.
            if unsafe { GetNumaNodeProcessorMask(raw_node as u8, &mut mask) } == 0 || mask == 0 {
                continue;
            }
            let node_id = NumaNodeId::new(raw_node);
            let mut processors = Vec::new();
            for processor in 0..64u32 {
                if (mask & (1u64 << processor)) != 0 {
                    processors.push(processor);
                    processor_node_pairs.push((processor, node_id));
                    logical_processors = logical_processors.max(processor as usize + 1);
                }
            }
            numa_nodes.push(NumaNode {
                id: node_id,
                processors: processors.into_boxed_slice(),
                distances: (0..node_count)
                    .map(|index| if index == raw_node as usize { 10 } else { 20 })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                memory_tier: MemoryTier::Dram,
            });
        }

        if numa_nodes.is_empty() {
            return Some(Self::single_node(logical_processor_count()));
        }

        Some(Self {
            epoch: TopologyEpoch::INITIAL,
            node_to_index: build_node_to_index(&numa_nodes),
            adjacent_nodes: build_adjacent_nodes(&numa_nodes),
            numa_nodes: numa_nodes.into_boxed_slice(),
            processor_to_node: build_processor_to_node(
                logical_processors.max(1),
                &processor_node_pairs,
            ),
            logical_processors: logical_processors.max(1),
            cache_levels: default_cache_levels(logical_processors.max(1)),
        })
    }
}

pub(super) fn build_processor_to_node(
    logical_processors: usize,
    mappings: &[(u32, NumaNodeId)],
) -> Box<[Option<NumaNodeId>]> {
    let max_processor = mappings
        .iter()
        .map(|(processor, _)| *processor as usize)
        .max()
        .unwrap_or(0);
    let mut processor_to_node = vec![None; logical_processors.max(max_processor + 1).max(1)];
    for (processor, node) in mappings {
        processor_to_node[*processor as usize] = Some(*node);
    }
    processor_to_node.into_boxed_slice()
}

pub(super) fn build_node_to_index(nodes: &[NumaNode]) -> Box<[Option<usize>]> {
    let max_node = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let mut node_to_index = vec![None; max_node + 1];
    for (index, node) in nodes.iter().enumerate() {
        node_to_index[node.id.index()] = Some(index);
    }
    node_to_index.into_boxed_slice()
}

pub(super) fn build_adjacent_nodes(nodes: &[NumaNode]) -> Box<[Box<[NumaNodeId]>]> {
    nodes
        .iter()
        .enumerate()
        .map(|(from_index, from_node)| {
            let mut adjacent: Vec<(NumaNodeId, u32)> = nodes
                .iter()
                .enumerate()
                .filter(|(to_index, _)| *to_index != from_index)
                .map(|(to_index, to_node)| {
                    let distance = from_node
                        .distances
                        .get(to_index)
                        .copied()
                        .unwrap_or(if from_node.id == to_node.id { 10 } else { 20 });
                    (to_node.id, distance)
                })
                .collect();
            adjacent.sort_by_key(|(_, distance)| *distance);
            adjacent
                .into_iter()
                .map(|(node_id, _)| node_id)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[cfg(all(feature = "std", target_os = "linux"))]
fn parse_cpu_list(cpulist: &str) -> Vec<u32> {
    let mut processors = Vec::new();
    for part in cpulist.trim().split(',') {
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (start.parse::<u32>(), end.parse::<u32>()) {
                processors.extend(start..=end);
            }
        } else if let Ok(processor) = part.parse::<u32>() {
            processors.push(processor);
        }
    }
    processors
}

pub(super) fn default_cache_levels(logical_processors: usize) -> Box<[CacheLevel]> {
    let processors: Vec<u32> = (0..logical_processors.max(1) as u32).collect();
    vec![
        CacheLevel {
            level: 1,
            size_bytes: 32 * 1024,
            shared_processors: Box::default(),
        },
        CacheLevel {
            level: 2,
            size_bytes: 256 * 1024,
            shared_processors: Box::default(),
        },
        CacheLevel {
            level: 3,
            size_bytes: 8 * 1024 * 1024,
            shared_processors: processors.into_boxed_slice(),
        },
    ]
    .into_boxed_slice()
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
