//! Shared fixtures for the branded placement-scope test files.
//!
//! [`synthetic_topology`] builds a single-processor-per-node topology, and the
//! `node_ids_via_*` helpers read it back through the split entry points, so
//! the scope tests and the split tests assert against one construction.

use themis::{
    build_processor_to_node, sync_region_placement_scope, CpuTopology, MemoryTier, NumaNode,
    NumaNodeId, TopologyEpoch,
};

/// A synthetic `CpuTopology` of `node_count` single-processor nodes with
/// distinct ids `0..node_count`.
pub fn synthetic_topology(node_count: usize) -> CpuTopology {
    let node_count_u32 = u32::try_from(node_count).expect("test topologies stay small");
    let nodes: Vec<NumaNode> = (0..node_count_u32)
        .map(|id| NumaNode {
            id: NumaNodeId::new(id),
            processors: vec![id].into_boxed_slice(),
            distances: vec![10; node_count].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        })
        .collect();
    let mappings: Vec<(u32, NumaNodeId)> = (0..node_count_u32)
        .map(|id| (id, NumaNodeId::new(id)))
        .collect();
    let processor_to_node = build_processor_to_node(node_count, &mappings);
    CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        node_count,
        None,
    )
}

pub fn node_ids_via_split(topology: &CpuTopology) -> Vec<u32> {
    sync_region_placement_scope(|placement| {
        placement
            .split(topology)
            .iter()
            .map(|p| p.node_id().get())
            .collect()
    })
}

pub fn node_ids_via_split_with(topology: &CpuTopology) -> Vec<u32> {
    sync_region_placement_scope(|placement| {
        placement.split_with(topology, |permits| {
            permits.iter().map(|p| p.node_id().get()).collect()
        })
    })
}
