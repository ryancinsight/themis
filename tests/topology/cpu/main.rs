//! CPU topology tests: value tests here, adjacency properties and view
//! behaviour in the sibling modules.

mod adjacency_properties;
mod views;

use themis::{
    build_default_distance_row, build_node_to_index, build_processor_to_node, CpuTopology,
    EfficiencyClass, MemoryTier, NumaNode, NumaNodeId, TopologyEpoch,
};
#[cfg(windows)]
use themis::{ProcessorAffinityGroups, ProcessorGroupAffinity};

// ACPI SLIT encodes local access as 10; Themis's documented fallback uses 20
// for a remote node when the platform provides no measured distance.
const LOCAL_DISTANCE: u32 = 10;
const REMOTE_DISTANCE: u32 = 20;

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
fn single_node_does_not_fabricate_cache_levels() {
    assert_eq!(CpuTopology::single_node(4).cache_levels(), None);
}

#[test]
#[cfg(windows)]
fn processor_affinity_groups_own_flattened_group_numbering() {
    let highest_native_bit = usize::BITS - 1;
    let affinity =
        ProcessorAffinityGroups::from_processors([64, 0, highest_native_bit, 66, 64, u32::MAX, 0]);

    let groups: Vec<(u16, usize)> = affinity
        .groups()
        .iter()
        .map(|group| (group.group(), group.mask()))
        .collect();
    assert_eq!(
        groups,
        [(0, 1 | (1usize << highest_native_bit)), (1, 0b0101),]
    );
    assert_eq!(affinity.unassigned_processors(), &[u32::MAX]);
    assert_eq!(affinity.assigned_processor_count(), 4);
    assert_eq!(affinity.requested_processor_count(), 5);
    assert!(!affinity.is_complete());
}

#[test]
#[cfg(windows)]
fn largest_processor_group_is_stable_on_equal_populations() {
    let affinity = ProcessorAffinityGroups::from_processors([66, 64, 2, 0]);
    assert_eq!(
        affinity.group(0).map(ProcessorGroupAffinity::mask),
        Some(0b0101)
    );
    assert_eq!(
        affinity.group(1).map(ProcessorGroupAffinity::mask),
        Some(0b0101)
    );
    assert_eq!(
        affinity.largest_group().map(ProcessorGroupAffinity::group),
        Some(0)
    );
}

#[test]
fn efficiency_view_discharges_presence_once_for_group_affinity() {
    let classes: Box<[EfficiencyClass]> = (0..70u32)
        .map(|processor| {
            if matches!(processor, 1 | 65 | 66) {
                EfficiencyClass::new(1)
            } else {
                EfficiencyClass::LOWEST
            }
        })
        .collect();
    let topology = CpuTopology::single_node(70).with_efficiency_classes_for_test(Some(classes));
    let efficiency = topology
        .efficiency()
        .expect("the fixture reports complete efficiency classes");

    assert_eq!(efficiency.class_count(), 2);
    assert!(efficiency.is_hybrid());
    assert_eq!(efficiency.highest_class(), EfficiencyClass::new(1));
    assert_eq!(
        efficiency.highest_class_processors().collect::<Vec<_>>(),
        [1, 65, 66]
    );

    #[cfg(windows)]
    {
        let affinity = efficiency.highest_class_affinity_groups();
        let groups: Vec<(u16, usize)> = affinity
            .groups()
            .iter()
            .map(|group| (group.group(), group.mask()))
            .collect();
        assert_eq!(groups, [(0, 0b10), (1, 0b110)]);
        assert!(affinity.is_complete());
        assert_eq!(affinity.requested_processor_count(), 3);
    }
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
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(2)), (1, NumaNodeId::new(7))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

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

#[test]
fn raw_indexed_sparse_node_ids_resolve_correct_distances() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(2),
            processors: vec![0].into_boxed_slice(),
            distances: vec![20, 20, LOCAL_DISTANCE, 20, 20, 20, 20, 45].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(7),
            processors: vec![1].into_boxed_slice(),
            distances: vec![20, 20, 45, 20, 20, 20, 20, LOCAL_DISTANCE].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(2)), (1, NumaNodeId::new(7))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

    assert_eq!(
        topology.distance(NumaNodeId::new(2), NumaNodeId::new(7)),
        45
    );
    assert_eq!(
        topology.distance(NumaNodeId::new(7), NumaNodeId::new(2)),
        45
    );
    assert_eq!(
        topology.adjacent_nodes(NumaNodeId::new(2)),
        &[NumaNodeId::new(7)]
    );
}

#[test]
fn compact_distance_indexing_with_shifted_node_ids() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![0].into_boxed_slice(),
            distances: vec![LOCAL_DISTANCE, 35].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(2),
            processors: vec![1].into_boxed_slice(),
            distances: vec![35, LOCAL_DISTANCE].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(1)), (1, NumaNodeId::new(2))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

    assert_eq!(
        topology.distance(NumaNodeId::new(2), NumaNodeId::new(1)),
        35
    );
    assert_eq!(
        topology.distance(NumaNodeId::new(1), NumaNodeId::new(2)),
        35
    );
}

#[test]
#[should_panic(expected = "invariant check failed: duplicate NUMA node ID")]
fn duplicate_node_ids_panics_on_construction() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![0].into_boxed_slice(),
            distances: vec![LOCAL_DISTANCE, LOCAL_DISTANCE].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![1].into_boxed_slice(),
            distances: vec![LOCAL_DISTANCE, LOCAL_DISTANCE].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let _node_index = build_node_to_index(&nodes);
}

// ── Property / differential coverage for the table builders ──
//
