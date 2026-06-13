//! Windows NUMA API CPU topology detection.

use super::super::{
    build_adjacent_nodes, build_node_to_index, build_processor_to_node, default_cache_levels,
    logical_processor_count, CpuTopology,
};
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};
use crate::topology::types::NumaNode;

pub(super) fn detect() -> Option<CpuTopology> {
    extern "system" {
        fn GetNumaHighestNodeNumber(highest_node_number: *mut u32) -> i32;
        fn GetNumaNodeProcessorMask(node: u8, processor_mask: *mut u64) -> i32;
    }

    let mut highest_node = 0u32;
    // SAFETY: The API writes one `u32` through a valid output pointer.
    if unsafe { GetNumaHighestNodeNumber(&mut highest_node) } == 0 {
        return Some(CpuTopology::single_node(logical_processor_count()));
    }

    let node_count = highest_node.saturating_add(1) as usize;
    let mut numa_nodes = Vec::with_capacity(node_count);
    let mut processor_node_pairs = Vec::with_capacity(logical_processor_count());
    let mut logical_processors = 0usize;

    for raw_node in 0..=highest_node {
        let mut mask = 0u64;
        // SAFETY: The API writes one processor mask through a valid pointer.
        if unsafe { GetNumaNodeProcessorMask(raw_node as u8, &mut mask) } == 0 || mask == 0 {
            continue;
        }
        let node_id = NumaNodeId::new(raw_node);
        let mut processors = Vec::with_capacity(mask.count_ones() as usize);
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
        return Some(CpuTopology::single_node(logical_processor_count()));
    }

    Some(CpuTopology {
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
