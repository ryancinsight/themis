//! Linux sysfs CPU topology detection.

use super::super::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
    detect_cache_levels, detect_efficiency_classes, logical_processor_count, parse_cpu_list,
    CpuTopology,
};
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};
use crate::topology::types::NumaNode;
use std::fs;

// `CpuTopology::detect` exposes `Option<CpuTopology>` publicly: it models
// "no backend could produce a topology". Every current backend resolves to at
// least a single-node snapshot, but diverging one backend's signature would
// fork the seam that public contract sits on.
#[expect(
    clippy::unnecessary_wraps,
    reason = "platform detect backends share the Option-returning contract of CpuTopology::detect"
)]
pub(super) fn detect() -> Option<CpuTopology> {
    let nodes_path = "/sys/devices/system/node/";
    let node_entries = fs::read_dir(nodes_path).ok();
    let mut node_ids: Vec<u32> = node_entries
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| name.strip_prefix("node")?.parse::<u32>().ok())
        .filter(|&id| id < 1024)
        .take(1024)
        .collect();
    node_ids.sort_unstable();

    if node_ids.is_empty() {
        return Some(CpuTopology::single_node(logical_processor_count()));
    }

    let mut numa_nodes = Vec::with_capacity(node_ids.len());
    let mut processor_node_pairs = Vec::with_capacity(logical_processor_count());

    for (from_index, node_id_raw) in node_ids.iter().enumerate() {
        let node_id = NumaNodeId::new(*node_id_raw);
        let cpulist_path = format!("{nodes_path}/node{node_id_raw}/cpulist");
        let processors = fs::read_to_string(cpulist_path)
            .map(|value| parse_cpu_list(&value))
            .unwrap_or_default();

        for processor in &processors {
            processor_node_pairs.push((*processor, node_id));
        }

        let distance_path = format!("{nodes_path}/node{node_id_raw}/distance");
        let distances = fs::read_to_string(distance_path).map_or_else(
            |_| build_default_distance_row(node_ids.len(), from_index).into_vec(),
            |value| {
                value
                    .split_whitespace()
                    .filter_map(|part| part.parse::<u32>().ok())
                    .take(1024)
                    .collect::<Vec<_>>()
            },
        );

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
    Some(CpuTopology {
        epoch: TopologyEpoch::INITIAL,
        numa_nodes: numa_nodes.into_boxed_slice(),
        node_to_index,
        processor_to_node,
        adjacent_nodes,
        logical_processors,
        cache_levels: detect_cache_levels(logical_processors),
        efficiency_classes: detect_efficiency_classes(logical_processors),
    })
}
