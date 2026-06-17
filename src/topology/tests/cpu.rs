//! CPU topology unit tests.

use super::super::cpu::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
    default_cache_levels, CpuTopology, LOCAL_DISTANCE, REMOTE_DISTANCE,
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
fn default_cache_levels_share_only_last_level() {
    let levels = default_cache_levels(4);

    assert_eq!(levels.len(), 3);
    assert_eq!(levels[0].level, 1);
    assert_eq!(levels[0].size_bytes, 32 * 1024);
    assert_eq!(levels[0].shared_processors.as_ref(), &[]);
    assert_eq!(levels[1].level, 2);
    assert_eq!(levels[1].size_bytes, 256 * 1024);
    assert_eq!(levels[1].shared_processors.as_ref(), &[]);
    assert_eq!(levels[2].level, 3);
    assert_eq!(levels[2].size_bytes, 8 * 1024 * 1024);
    assert_eq!(levels[2].shared_processors.as_ref(), &[0, 1, 2, 3]);
}

#[test]
fn distance_defaults_preserve_self_and_remote_values() {
    let topology = CpuTopology::single_node(1);
    assert_eq!(
        topology.distance(NumaNodeId::ZERO, NumaNodeId::ZERO),
        LOCAL_DISTANCE
    );
    assert_eq!(
        topology.distance(NumaNodeId::ZERO, NumaNodeId::new(9)),
        REMOTE_DISTANCE
    );
}

#[test]
fn default_distance_rows_preserve_local_and_remote_costs() {
    assert_eq!(
        build_default_distance_row(4, 2).as_ref(),
        &[
            REMOTE_DISTANCE,
            REMOTE_DISTANCE,
            LOCAL_DISTANCE,
            REMOTE_DISTANCE
        ]
    );
}

#[test]
fn detected_topology_has_queryable_first_node() {
    let topology = CpuTopology::detect().expect("topology detection should return fallback");
    let first_node = topology
        .numa_nodes()
        .first()
        .expect("invariant: detection fallback always returns at least one NUMA node");

    assert_eq!(topology.node_index(first_node.id), Some(0));
    assert_eq!(
        topology.distance(first_node.id, first_node.id),
        LOCAL_DISTANCE
    );
    assert_eq!(
        topology.adjacent_nodes(first_node.id).len(),
        topology.numa_nodes().len().saturating_sub(1)
    );
}

#[test]
fn sparse_node_ids_use_compact_distance_rows() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(2),
            processors: vec![0].into_boxed_slice(),
            distances: vec![LOCAL_DISTANCE, 31].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(7),
            processors: vec![1].into_boxed_slice(),
            distances: vec![31, LOCAL_DISTANCE].into_boxed_slice(),
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
