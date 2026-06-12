//! Platform CPU topology detection.

#[cfg(any(
    all(feature = "std", target_os = "linux"),
    all(feature = "std", windows)
))]
use super::{
    build_adjacent_nodes, build_node_to_index, build_processor_to_node, default_cache_levels,
};
use super::{logical_processor_count, CpuTopology};
#[cfg(any(
    all(feature = "std", target_os = "linux"),
    all(feature = "std", windows)
))]
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};
#[cfg(any(
    all(feature = "std", target_os = "linux"),
    all(feature = "std", windows)
))]
use crate::topology::types::NumaNode;

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
