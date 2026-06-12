//! CPU topology unit tests.

use super::super::cpu::{
    build_adjacent_nodes, build_node_to_index, build_processor_to_node, default_cache_levels,
    CpuTopology,
};
use super::super::types::NumaNode;
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[test]
fn single_node_maps_every_processor_to_node_zero() {
    let topology = CpuTopology::single_node(4);
    assert_eq!(topology.numa_nodes().len(), 1);
    assert_eq!(topology.logical_processors(), 4);
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
    assert!(!topology.numa_nodes().is_empty());
    assert!(topology.logical_processors() > 0);
}

#[test]
fn sparse_node_ids_use_compact_distance_rows() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(2),
            processors: vec![0].into_boxed_slice(),
            distances: vec![10, 31].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(7),
            processors: vec![1].into_boxed_slice(),
            distances: vec![31, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let topology = CpuTopology {
        epoch: TopologyEpoch::INITIAL,
        processor_to_node: build_processor_to_node(
            2,
            &[(0, NumaNodeId::new(2)), (1, NumaNodeId::new(7))],
        ),
        node_to_index: build_node_to_index(&nodes),
        adjacent_nodes: build_adjacent_nodes(&nodes),
        numa_nodes: nodes.into_boxed_slice(),
        logical_processors: 2,
        cache_levels: default_cache_levels(2),
    };

    assert_eq!(topology.processor_to_numa_node(1), Some(NumaNodeId::new(7)));
    assert_eq!(
        topology.distance(NumaNodeId::new(2), NumaNodeId::new(7)),
        31
    );
    assert_eq!(topology.node_index(NumaNodeId::new(7)), Some(1));
    assert_eq!(
        topology.adjacent_nodes(NumaNodeId::new(2)),
        &[NumaNodeId::new(7)]
    );
}
