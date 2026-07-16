//! CPU topology unit tests.

use themis::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
    CpuTopology, MemoryTier, NumaNode, NumaNodeId, TopologyEpoch,
};
use themis::{LOCAL_DISTANCE, REMOTE_DISTANCE};

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
// `build_adjacent_nodes` has two code paths selected by node count against a
// `STACK_LIMIT` of 128 (a fixed stack array for the common small case, a heap
// `Vec` above it). Both must implement identical adjacency semantics. The
// reference oracle below re-derives the expected flattened adjacency with the
// same distance-lookup rule and stable ordering, so comparing the builder's
// output against it validates *both* paths against one specification.

use proptest::prelude::*;

/// Mirror of `default_distance` (self = local, other = remote), expressed
/// through the public distance constants.
fn ref_default_distance(from_index: usize, to_index: usize) -> u32 {
    if from_index == to_index {
        LOCAL_DISTANCE
    } else {
        REMOTE_DISTANCE
    }
}

/// Independent reference implementation of `build_adjacent_nodes`: for every
/// node, the ids of all other nodes ordered by non-decreasing distance under
/// the builder's own index rule (dense-by-id when `distances.len()` exceeds the
/// max node index, otherwise compact-by-position), falling back to the default
/// distance, with a stable sort so equal distances keep positional order.
fn ref_adjacent_nodes(nodes: &[NumaNode]) -> Vec<NumaNodeId> {
    if nodes.len() <= 1 {
        return Vec::new();
    }
    let max_node_id = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let mut flat = Vec::new();
    for (from_index, from_node) in nodes.iter().enumerate() {
        let mut adjacent: Vec<(NumaNodeId, u32)> = nodes
            .iter()
            .enumerate()
            .filter(|(to_index, _)| *to_index != from_index)
            .map(|(to_index, to_node)| {
                let idx = if from_node.distances.len() > max_node_id {
                    to_node.id.index()
                } else {
                    to_index
                };
                let distance = from_node
                    .distances
                    .get(idx)
                    .copied()
                    .unwrap_or(ref_default_distance(from_index, to_index));
                (to_node.id, distance)
            })
            .collect();
        adjacent.sort_by_key(|(_, distance)| *distance);
        flat.extend(adjacent.into_iter().map(|(id, _)| id));
    }
    flat
}

/// Distinct-id `NumaNode` sets with arbitrary distance-row layouts. Ids stay
/// small (< 16) so `distances` lengths in `0..=20` straddle `max_node_id`,
/// exercising both the dense-by-id and compact-by-position index rules.
fn arb_numa_nodes() -> impl Strategy<Value = Vec<NumaNode>> {
    proptest::collection::hash_set(0u32..16, 1..12).prop_flat_map(|id_set| {
        let ids: Vec<u32> = id_set.into_iter().collect();
        let count = ids.len();
        proptest::collection::vec(proptest::collection::vec(10u32..40, 0..=20), count).prop_map(
            move |dist_rows| {
                ids.iter()
                    .zip(dist_rows)
                    .map(|(&id, distances)| NumaNode {
                        id: NumaNodeId::new(id),
                        processors: Box::default(),
                        distances: distances.into_boxed_slice(),
                        memory_tier: MemoryTier::Dram,
                    })
                    .collect()
            },
        )
    })
}

proptest! {
    /// The stack-path builder output matches the independent reference for all
    /// small (≤ `STACK_LIMIT`) node sets and distance layouts.
    #[test]
    fn adjacent_nodes_matches_reference(nodes in arb_numa_nodes()) {
        prop_assert_eq!(
            build_adjacent_nodes(&nodes),
            ref_adjacent_nodes(&nodes).into_boxed_slice()
        );
    }

    /// Structural invariant: each node's adjacency row is a permutation of all
    /// *other* node ids — no self, no duplicates, every other node present.
    #[test]
    fn adjacent_nodes_row_is_permutation_of_others(nodes in arb_numa_nodes()) {
        let flat = build_adjacent_nodes(&nodes);
        let n = nodes.len();
        if n <= 1 {
            prop_assert!(flat.is_empty());
            return Ok(());
        }
        let stride = n - 1;
        prop_assert_eq!(flat.len(), n * stride);
        for (from_index, from_node) in nodes.iter().enumerate() {
            let row = &flat[from_index * stride..(from_index + 1) * stride];
            let mut got: Vec<u32> = row.iter().map(|id| id.get()).collect();
            got.sort_unstable();
            let mut expected: Vec<u32> = nodes
                .iter()
                .enumerate()
                .filter(|(to_index, _)| *to_index != from_index)
                .map(|(_, node)| node.id.get())
                .collect();
            expected.sort_unstable();
            prop_assert_eq!(&got, &expected, "row {} not a permutation of other ids", from_node.id.get());
        }
    }

    /// `build_node_to_index` round-trips: the index stored for each node's id
    /// resolves back to that node's position.
    #[test]
    fn node_to_index_round_trips(nodes in arb_numa_nodes()) {
        let node_to_index = build_node_to_index(&nodes);
        for (index, node) in nodes.iter().enumerate() {
            prop_assert_eq!(node_to_index[node.id.index()], index);
        }
    }

    /// `build_processor_to_node` places every mapped processor at its id and
    /// leaves all other slots `INVALID`; length covers both operands.
    #[test]
    fn processor_to_node_places_mappings(
        pairs in proptest::collection::hash_map(0u32..64, 0u32..8, 0..8),
        logical in 0usize..80,
    ) {
        let mappings: Vec<(u32, NumaNodeId)> =
            pairs.iter().map(|(&p, &n)| (p, NumaNodeId::new(n))).collect();
        let table = build_processor_to_node(logical, &mappings);
        let max_processor = mappings.iter().map(|(p, _)| *p as usize).max().unwrap_or(0);
        prop_assert!(table.len() >= logical);
        prop_assert!(table.len() > max_processor);
        for (proc, node) in &mappings {
            prop_assert_eq!(table[*proc as usize], *node);
        }
        for (proc, slot) in table.iter().enumerate() {
            if !mappings.iter().any(|(p, _)| *p as usize == proc) {
                prop_assert_eq!(*slot, NumaNodeId::INVALID);
            }
        }
    }
}

/// Heap path: node counts above `STACK_LIMIT` (128) take the `Vec`-based branch
/// of `build_adjacent_nodes`, which must agree with the same reference oracle.
#[test]
fn adjacent_nodes_heap_path_matches_reference() {
    let nodes: Vec<NumaNode> = (0u32..130)
        .map(|id| NumaNode {
            id: NumaNodeId::new(id),
            processors: vec![id].into_boxed_slice(),
            // Dense-by-id distances: nearer ids are closer, so each row has a
            // non-trivial ordering rather than a uniform remote distance.
            distances: (0u32..130)
                .map(|other| LOCAL_DISTANCE + other.abs_diff(id))
                .collect(),
            memory_tier: MemoryTier::Dram,
        })
        .collect();
    assert_eq!(
        build_adjacent_nodes(&nodes),
        ref_adjacent_nodes(&nodes).into_boxed_slice(),
        "heap-path adjacency diverges from reference"
    );
}
