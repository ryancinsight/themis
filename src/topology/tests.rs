//! Topology unit tests.

use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};
use super::cpu::{
    build_adjacent_nodes, build_node_to_index, build_processor_to_node, default_cache_levels,
    CpuTopology,
};
use super::gpu::GpuTopology;
use super::types::{GpuDeviceProperties, NumaNode};

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

mod gpu_topology_tests {
    use super::*;

    fn sample_properties() -> GpuDeviceProperties {
        GpuDeviceProperties {
            compute_units: 46,
            warp_width: 32,
            max_threads_per_unit: 1536,
            registers_per_unit: 65536,
            shared_mem_per_unit_bytes: 102_400,
            l2_bytes: 4 * 1024 * 1024,
            memory_tier: MemoryTier::Gddr,
            memory_bytes: 8 * 1024 * 1024 * 1024,
        }
    }

    #[test]
    fn provider_snapshot_round_trips_every_field() {
        let topology = GpuTopology::from_provider(sample_properties());
        assert_eq!(topology.compute_units(), 46);
        assert_eq!(topology.warp_width(), 32);
        assert_eq!(topology.max_threads_per_unit(), 1536);
        assert_eq!(topology.registers_per_unit(), 65536);
        assert_eq!(topology.shared_mem_per_unit_bytes(), 102_400);
        assert_eq!(topology.l2_bytes(), 4 * 1024 * 1024);
        assert_eq!(topology.memory_tier(), MemoryTier::Gddr);
        assert_eq!(topology.memory_bytes(), 8 * 1024 * 1024 * 1024);
        assert_eq!(topology.epoch(), TopologyEpoch::INITIAL);
    }

    #[test]
    fn max_resident_warps_is_units_times_threads_over_width() {
        let topology = GpuTopology::from_provider(sample_properties());
        // 46 * 1536 / 32 = 2208
        assert_eq!(topology.max_resident_warps(), 2208);

        let mut zero_width = sample_properties();
        zero_width.warp_width = 0;
        assert_eq!(
            GpuTopology::from_provider(zero_width).max_resident_warps(),
            0
        );
    }

    #[test]
    fn budgeted_tiers_are_not_host_allocatable() {
        assert!(!MemoryTier::Registers.is_host_allocatable());
        assert!(!MemoryTier::SharedMem.is_host_allocatable());
        assert!(MemoryTier::Gddr.is_host_allocatable());
        assert!(MemoryTier::HostPinned.is_host_allocatable());
        assert!(MemoryTier::Hbm.is_host_allocatable());
        assert!(MemoryTier::Dram.is_host_allocatable());
    }
}
