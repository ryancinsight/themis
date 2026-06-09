//! CPU and memory topology snapshots.

use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};

#[cfg(feature = "std")]
use std::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// NUMA node topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumaNode {
    /// Node identifier.
    pub id: NumaNodeId,
    /// Logical processors assigned to this node.
    pub processors: Vec<u32>,
    /// Relative distance to other nodes.
    pub distances: Vec<u32>,
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
    /// Processors sharing this cache.
    pub shared_processors: Vec<u32>,
}

/// CPU topology snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Snapshot epoch.
    pub epoch: TopologyEpoch,
    /// NUMA nodes.
    pub numa_nodes: Vec<NumaNode>,
    /// Processor to NUMA node mapping.
    pub processor_to_node: BTreeMap<u32, NumaNodeId>,
    /// Logical processor count.
    pub logical_processors: usize,
    /// Cache hierarchy.
    pub cache_levels: Vec<CacheLevel>,
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
        let mut processor_to_node = BTreeMap::new();
        for processor in &processors {
            processor_to_node.insert(*processor, node_id);
        }

        Self {
            epoch: TopologyEpoch::INITIAL,
            numa_nodes: vec![NumaNode {
                id: node_id,
                processors: processors.clone(),
                distances: vec![10],
                memory_tier: MemoryTier::Dram,
            }],
            processor_to_node,
            logical_processors,
            cache_levels: default_cache_levels(logical_processors),
        }
    }

    /// Returns the NUMA node for a processor.
    #[must_use]
    pub fn processor_to_numa_node(&self, processor: u32) -> Option<NumaNodeId> {
        self.processor_to_node.get(&processor).copied()
    }

    /// Returns node distance.
    #[must_use]
    pub fn distance(&self, from: NumaNodeId, to: NumaNodeId) -> u32 {
        self.numa_nodes
            .iter()
            .find(|node| node.id == from)
            .and_then(|node| node.distances.get(to.index()).copied())
            .unwrap_or(if from == to { 10 } else { 20 })
    }

    /// Returns adjacent nodes sorted by distance.
    #[must_use]
    pub fn adjacent_nodes(&self, node_id: NumaNodeId) -> Vec<NumaNodeId> {
        let mut adjacent: Vec<(NumaNodeId, u32)> = self
            .numa_nodes
            .iter()
            .filter(|node| node.id != node_id)
            .map(|node| (node.id, self.distance(node_id, node.id)))
            .collect();
        adjacent.sort_by_key(|(_, distance)| *distance);
        adjacent.into_iter().map(|(id, _)| id).collect()
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
        let mut processor_to_node = BTreeMap::new();

        for node_id_raw in &node_ids {
            let node_id = NumaNodeId::new(*node_id_raw);
            let cpulist_path = format!("{nodes_path}/node{node_id_raw}/cpulist");
            let processors = fs::read_to_string(cpulist_path)
                .map(|value| parse_cpu_list(&value))
                .unwrap_or_default();

            for processor in &processors {
                processor_to_node.insert(*processor, node_id);
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
                processors,
                distances,
                memory_tier: MemoryTier::Dram,
            });
        }

        let logical_processors = logical_processor_count();
        Some(Self {
            epoch: TopologyEpoch::INITIAL,
            numa_nodes,
            processor_to_node,
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
        let mut processor_to_node = BTreeMap::new();
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
                    processor_to_node.insert(processor, node_id);
                    logical_processors = logical_processors.max(processor as usize + 1);
                }
            }
            numa_nodes.push(NumaNode {
                id: node_id,
                processors,
                distances: (0..node_count)
                    .map(|index| if index == raw_node as usize { 10 } else { 20 })
                    .collect(),
                memory_tier: MemoryTier::Dram,
            });
        }

        if numa_nodes.is_empty() {
            return Some(Self::single_node(logical_processor_count()));
        }

        Some(Self {
            epoch: TopologyEpoch::INITIAL,
            numa_nodes,
            processor_to_node,
            logical_processors: logical_processors.max(1),
            cache_levels: default_cache_levels(logical_processors.max(1)),
        })
    }
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

fn default_cache_levels(logical_processors: usize) -> Vec<CacheLevel> {
    let processors: Vec<u32> = (0..logical_processors.max(1) as u32).collect();
    vec![
        CacheLevel {
            level: 1,
            size_bytes: 32 * 1024,
            shared_processors: Vec::new(),
        },
        CacheLevel {
            level: 2,
            size_bytes: 256 * 1024,
            shared_processors: Vec::new(),
        },
        CacheLevel {
            level: 3,
            size_bytes: 8 * 1024 * 1024,
            shared_processors: processors,
        },
    ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_node_maps_every_processor_to_node_zero() {
        let topology = CpuTopology::single_node(4);
        assert_eq!(topology.numa_nodes.len(), 1);
        assert_eq!(topology.logical_processors, 4);
        for processor in 0..4 {
            assert_eq!(
                topology.processor_to_numa_node(processor),
                Some(NumaNodeId::ZERO)
            );
        }
    }

    #[test]
    fn distance_defaults_preserve_self_and_remote_values() {
        let topology = CpuTopology::single_node(1);
        assert_eq!(topology.distance(NumaNodeId::ZERO, NumaNodeId::ZERO), 10);
        assert_eq!(topology.distance(NumaNodeId::ZERO, NumaNodeId::new(9)), 20);
    }

    #[test]
    fn detected_topology_has_at_least_one_node() {
        let topology = CpuTopology::detect().expect("topology detection should return fallback");
        assert!(!topology.numa_nodes.is_empty());
        assert!(topology.logical_processors > 0);
    }
}
