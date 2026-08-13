//! Branded placement scope unit tests.
//!
//! Test code is exempt from `clippy::unwrap_used`: a panic here is the
//! failure report, not a shipped panic path.
#![allow(clippy::unwrap_used)]

use themis::{
    build_processor_to_node, CpuTopology, MemoryTier, NumaNode, NumaNodeId, PlacementHint,
    TopologyEpoch,
};
use themis::{
    sync_region_placement_scope, thread_local_placement_scope, ConstNumaPinnedCell,
    ConstNumaPinnedCellRef, ConstNumaPinnedSlice, ConstNumaPinnedSliceRef, NumaPinnedCell,
    NumaPinnedCellRef, NumaPinnedSlice, NumaPinnedSliceRef,
};

#[test]
fn thread_local_scope_controls_placement_state() {
    let observed = thread_local_placement_scope(|mut placement| {
        let hint = placement.cell(PlacementHint::Current);
        *placement.write(&hint) = PlacementHint::Numa(NumaNodeId::new(3));
        *placement.read(&hint)
    });

    assert_eq!(observed, PlacementHint::Numa(NumaNodeId::new(3)));
}

#[test]
fn sync_region_scope_controls_portable_placement_state() {
    let observed = sync_region_placement_scope(|mut placement| {
        let tier = placement.cell(MemoryTier::Dram);
        *placement.write(&tier) = MemoryTier::Hbm;
        *placement.read(&tier)
    });

    assert_eq!(observed, MemoryTier::Hbm);
}

#[test]
fn sync_region_split_allows_parallel_node_access() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(0),
            processors: vec![0].into_boxed_slice(),
            distances: vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![1].into_boxed_slice(),
            distances: vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

    let (val0, val1) = sync_region_placement_scope(|placement| {
        let cell0 = NumaPinnedCell::new(NumaNodeId::new(0), 100u32);
        let cell1 = NumaPinnedCell::new(NumaNodeId::new(1), 200u32);

        let mut permits = placement.split(&topology);
        assert_eq!(permits.len(), 2);

        let mut permit0 = permits.remove(0);
        let mut permit1 = permits.remove(0);

        *permit0.write(&cell0).unwrap() = 111u32;
        *permit1.write(&cell1).unwrap() = 222u32;

        assert!(permit0.write(&cell1).is_none());
        assert!(permit1.write(&cell0).is_none());

        (
            *permit0.read(&cell0).unwrap(),
            *permit1.read(&cell1).unwrap(),
        )
    });

    assert_eq!(val0, 111);
    assert_eq!(val1, 222);
}

#[test]
fn sync_region_split_with_avoid_heap_allocations() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(0),
            processors: vec![0].into_boxed_slice(),
            distances: vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![1].into_boxed_slice(),
            distances: vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

    let (val0, val1) = sync_region_placement_scope(|placement| {
        let cell0 = NumaPinnedCell::new(NumaNodeId::new(0), 500u32);
        let cell1 = NumaPinnedCell::new(NumaNodeId::new(1), 600u32);

        placement.split_with(&topology, |permits| {
            assert_eq!(permits.len(), 2);
            let mut iter = permits.iter_mut();
            let permit0 = iter.next().unwrap();
            let permit1 = iter.next().unwrap();

            *permit0.write(&cell0).unwrap() = 555u32;
            *permit1.write(&cell1).unwrap() = 666u32;

            assert!(permit0.write(&cell1).is_none());
            assert!(permit1.write(&cell0).is_none());

            (
                *permit0.read(&cell0).unwrap(),
                *permit1.read(&cell1).unwrap(),
            )
        })
    });

    assert_eq!(val0, 555);
    assert_eq!(val1, 666);
}

#[test]
fn sync_region_pinned_slice_allows_efficient_bulk_access() {
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(0),
            processors: vec![0].into_boxed_slice(),
            distances: vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![1].into_boxed_slice(),
            distances: vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

    let (s0_sum, s1_sum) = sync_region_placement_scope(|placement| {
        let slice0 = NumaPinnedSlice::new(NumaNodeId::new(0), vec![1, 2, 3]);
        let slice1 = NumaPinnedSlice::new(NumaNodeId::new(1), vec![10, 20, 30]);

        let mut permits = placement.split(&topology);
        assert_eq!(permits.len(), 2);

        let mut permit0 = permits.remove(0);
        let mut permit1 = permits.remove(0);

        // Write to slice 0 with permit 0 (matching node ID)
        let s0 = permit0.write_slice(&slice0).unwrap();
        for x in s0.iter_mut() {
            *x *= 2;
        }

        // Try writing to slice 1 with permit 0 (mismatch node ID)
        assert!(permit0.write_slice(&slice1).is_none());

        // Write to slice 1 with permit 1 (matching node ID)
        let s1 = permit1.write_slice(&slice1).unwrap();
        for x in s1.iter_mut() {
            *x += 5;
        }

        // Try writing to slice 0 with permit 1 (mismatch node ID)
        assert!(permit1.write_slice(&slice0).is_none());

        let sum0 = permit0
            .read_slice(&slice0)
            .map(|s| s.iter().sum::<i32>())
            .unwrap();
        let sum1 = permit1
            .read_slice(&slice1)
            .map(|s| s.iter().sum::<i32>())
            .unwrap();
        (sum0, sum1)
    });

    assert_eq!(s0_sum, 12); // (1*2 + 2*2 + 3*2) = 12
    assert_eq!(s1_sum, 75); // (15 + 25 + 35) = 75
}

#[test]
fn pinned_slice_constructors_retain_branded_values() {
    let dynamic = sync_region_placement_scope(|placement| {
        let topology = synthetic_topology(1);
        let dynamic = NumaPinnedSlice::new(NumaNodeId::new(0), vec![1_u32, 2, 3]);
        let mut permits = placement.split(&topology);
        let mut permit = permits.pop().expect("synthetic topology has one node");
        for value in permit.write_slice(&dynamic).unwrap().iter_mut() {
            *value += 10;
        }

        permit
            .read_slice(&dynamic)
            .expect("matching dynamic node permit")
            .to_vec()
    });

    let static_slice = sync_region_placement_scope(|placement| {
        let static_slice = ConstNumaPinnedSlice::<0, u32>::new(vec![4, 5, 6]);
        let mut permit = placement.project_static::<0>();

        for value in permit.write_slice(&static_slice).iter_mut() {
            *value += 20;
        }
        permit.read_slice(&static_slice).to_vec()
    });

    assert_eq!(dynamic, [11, 12, 13]);
    assert_eq!(static_slice, [24, 25, 26]);
}

#[test]
fn pinned_slice_from_fn_and_partition_paths_use_melinoe_collections() {
    let dynamic = sync_region_placement_scope(|placement| {
        let mut dynamic =
            NumaPinnedSlice::from_fn(NumaNodeId::new(0), 16, |index| usize::MAX - index);
        let topology = synthetic_topology(1);
        let mut permits = placement.split(&topology);
        let mut permit = permits.pop().expect("synthetic topology has one node");
        // More requested partitions than elements exercises Melinoe's
        // partition-count clamping without changing Themis's ownership
        // boundary.
        let plan = melinoe::sync::PartitionPlan::parts(64);
        let visited = std::sync::atomic::AtomicUsize::new(0);

        permit
            .partition_for_each_mut_with(&mut dynamic, plan, |start, values| {
                for (offset, value) in values.iter_mut().enumerate() {
                    visited.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    *value = start + offset;
                }
            })
            .expect("matching dynamic node permit");
        assert_eq!(visited.load(std::sync::atomic::Ordering::Relaxed), 16);

        let mut mismatched = NumaPinnedSlice::from_fn(NumaNodeId::new(1), 4, |index| index);
        assert!(permit
            .partition_for_each_mut_with(&mut mismatched, plan, |_, _| {})
            .is_none());

        let mut empty = NumaPinnedSlice::from_fn(NumaNodeId::new(0), 0, |_| 0usize);
        let empty_invocations = std::sync::atomic::AtomicUsize::new(0);
        assert!(permit
            .partition_for_each_mut_with(&mut empty, plan, |_, _| {
                empty_invocations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            })
            .is_some());
        assert_eq!(
            empty_invocations.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(permit.read_slice(&empty).unwrap().len(), 0);

        let values = permit
            .read_slice(&dynamic)
            .expect("matching dynamic node permit");
        assert_eq!(values, (0..16).collect::<Vec<_>>().as_slice());
        values.iter().sum::<usize>()
    });

    let static_values = sync_region_placement_scope(|placement| {
        let mut static_values = ConstNumaPinnedSlice::<0, usize>::from_fn(16, |index| index * 10);
        let plan = melinoe::sync::PartitionPlan::parts(4);
        let mut permit = placement.project_static::<0>();
        permit.partition_for_each_mut_with(&mut static_values, plan, |start, values| {
            for (offset, value) in values.iter_mut().enumerate() {
                *value += start + offset;
            }
        });
        permit.read_slice(&static_values).iter().sum::<usize>()
    });

    assert_eq!(dynamic, (0..16).sum::<usize>());
    assert_eq!(
        static_values,
        (0..16).map(|index| index * 11).sum::<usize>()
    );
}

#[test]
fn const_numa_branding_provides_zero_cost_static_access() {
    let (val0, val1) = sync_region_placement_scope(|placement| {
        let cell0 = ConstNumaPinnedCell::<0, u32>::new(700);
        let cell1 = ConstNumaPinnedCell::<1, u32>::new(800);

        let (mut permit0, mut permit1) = placement.split_static::<0, 1>();

        *permit0.write(&cell0) = 777;
        *permit1.write(&cell1) = 888;

        (*permit0.read(&cell0), *permit1.read(&cell1))
    });

    assert_eq!(val0, 777);
    assert_eq!(val1, 888);
}

#[test]
fn const_numa_split_static_3_gives_three_disjoint_permits() {
    let (v0, v1, v2) = sync_region_placement_scope(|placement| {
        let cell0 = ConstNumaPinnedCell::<0, u32>::new(0);
        let cell1 = ConstNumaPinnedCell::<1, u32>::new(0);
        let cell2 = ConstNumaPinnedCell::<2, u32>::new(0);

        let (mut p0, mut p1, mut p2) = placement.split_static_3::<0, 1, 2>();
        *p0.write(&cell0) = 10;
        *p1.write(&cell1) = 20;
        *p2.write(&cell2) = 30;

        (*p0.read(&cell0), *p1.read(&cell1), *p2.read(&cell2))
    });

    assert_eq!((v0, v1, v2), (10, 20, 30));
}

#[test]
fn const_numa_split_static_4_gives_four_disjoint_permits() {
    let (v0, v1, v2, v3) = sync_region_placement_scope(|placement| {
        let cell0 = ConstNumaPinnedCell::<0, u32>::new(0);
        let cell1 = ConstNumaPinnedCell::<1, u32>::new(0);
        let cell2 = ConstNumaPinnedCell::<2, u32>::new(0);
        let cell3 = ConstNumaPinnedCell::<3, u32>::new(0);

        let (mut p0, mut p1, mut p2, mut p3) = placement.split_static_4::<0, 1, 2, 3>();
        *p0.write(&cell0) = 11;
        *p1.write(&cell1) = 22;
        *p2.write(&cell2) = 33;
        *p3.write(&cell3) = 44;

        (
            *p0.read(&cell0),
            *p1.read(&cell1),
            *p2.read(&cell2),
            *p3.read(&cell3),
        )
    });

    assert_eq!((v0, v1, v2, v3), (11, 22, 33, 44));
}

#[test]
fn const_numa_pinned_slices_support_direct_borrowing() {
    let sum = sync_region_placement_scope(|placement| {
        let slice = ConstNumaPinnedSlice::<0, u32>::new(vec![10, 20, 30]);
        let mut permit = placement.project_static::<0>();

        let elements = permit.write_slice(&slice);
        for x in elements.iter_mut() {
            *x += 1;
        }

        permit.read_slice(&slice).iter().sum::<u32>()
    });

    assert_eq!(sum, 63);
}

#[test]
fn thread_local_numa_placement_controls_local_access() {
    let (s0_val, s1_val) = thread_local_placement_scope(|placement| {
        let cell0 = NumaPinnedCell::new(NumaNodeId::new(0), 10u32);
        let cell1 = NumaPinnedCell::new(NumaNodeId::new(1), 20u32);

        let mut permit = placement.pin_local();
        let target_node = permit.node_id();

        let val0 = if target_node == NumaNodeId::new(0) {
            *permit.write(&cell0).unwrap() += 5;
            *permit.read(&cell0).unwrap()
        } else {
            assert!(permit.write(&cell0).is_none());
            10
        };

        let val1 = if target_node == NumaNodeId::new(1) {
            *permit.write(&cell1).unwrap() += 5;
            *permit.read(&cell1).unwrap()
        } else {
            assert!(permit.write(&cell1).is_none());
            20
        };

        (val0, val1)
    });

    assert!(s0_val == 15 || s0_val == 10);
    assert!(s1_val == 25 || s1_val == 20);
}

#[test]
fn const_thread_local_numa_placement_controls_static_access() {
    let (val0, _) = thread_local_placement_scope(|placement| {
        let cell0 = ConstNumaPinnedCell::<0, u32>::new(100);
        let mut permit = placement.pin_local_static::<0>();
        *permit.write(&cell0) += 50;
        (*permit.read(&cell0), 0)
    });

    let sum1_actual = thread_local_placement_scope(|placement| {
        let slice1 = ConstNumaPinnedSlice::<1, u32>::new(vec![1, 2, 3]);
        let mut permit = placement.pin_local_static::<1>();
        let slice = permit.write_slice(&slice1);
        for x in slice.iter_mut() {
            *x += 10;
        }
        permit.read_slice(&slice1).iter().sum::<u32>()
    });

    assert_eq!(val0, 150);
    assert_eq!(sum1_actual, 36);
}

#[test]
fn cell_and_slice_reference_types_avoid_allocations() {
    use melinoe::MelinoeCell;

    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(0),
            processors: vec![0].into_boxed_slice(),
            distances: vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(1),
            processors: vec![1].into_boxed_slice(),
            distances: vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))]);
    let topology = CpuTopology::new_for_test(
        TopologyEpoch::INITIAL,
        nodes.into_boxed_slice(),
        processor_to_node,
        2,
        None,
    );

    let (val, sum) = sync_region_placement_scope(|placement| {
        // Stack-allocated cells and arrays. The `&mut` borrow each reference
        // consumes is the placement proof: it is what stops a second node tag
        // being attached to the same cell.
        let mut cell_raw = MelinoeCell::new(42u32);
        let cell_ref = NumaPinnedCellRef::from_unique(NumaNodeId::new(0), &mut cell_raw);

        let mut array_raw = [
            MelinoeCell::new(10u32),
            MelinoeCell::new(20u32),
            MelinoeCell::new(30u32),
        ];
        let slice_ref = NumaPinnedSliceRef::from_unique(NumaNodeId::new(0), &mut array_raw);

        let mut permits = placement.split(&topology);
        let mut permit0 = permits.remove(0);

        *permit0.write(&cell_ref).unwrap() += 8;

        let s = permit0.write_slice(&slice_ref).unwrap();
        for x in s.iter_mut() {
            *x += 1;
        }

        let final_val = *permit0.read(&cell_ref).unwrap();
        let final_sum = permit0.read_slice(&slice_ref).unwrap().iter().sum::<u32>();

        (final_val, final_sum)
    });

    assert_eq!(val, 50);
    assert_eq!(sum, 63);
}

#[test]
fn const_cell_and_slice_reference_types_work() {
    use melinoe::MelinoeCell;

    let (val, sum) = sync_region_placement_scope(|placement| {
        let mut cell_raw = MelinoeCell::new(100u32);
        let cell_ref = ConstNumaPinnedCellRef::<5, u32>::from_unique(&mut cell_raw);

        let mut array_raw = [MelinoeCell::new(1u32), MelinoeCell::new(2u32)];
        let slice_ref = ConstNumaPinnedSliceRef::<5, u32>::from_unique(&mut array_raw);

        // `project_static` consumes the region and mints one capability, so
        // Melinoe's own one-token-per-brand rule still holds: no obligation,
        // no `unsafe`.
        let mut permit = placement.project_static::<5>();

        *permit.write(&cell_ref) += 50;

        let s = permit.write_slice(&slice_ref);
        for x in s.iter_mut() {
            *x *= 10;
        }

        let final_val = *permit.read(&cell_ref);
        let final_sum = permit.read_slice(&slice_ref).iter().sum::<u32>();

        (final_val, final_sum)
    });

    assert_eq!(val, 150);
    assert_eq!(sum, 30);
}

// ── `split` vs `split_with` differential coverage ──
//
// `split_with` has two code paths selected by node count against a
// `MAX_STACK_NODES` limit of 128: a `MaybeUninit` stack array with a manual
// `DropGuard` for the common small case, a heap `Vec` fallback above it. Both
// must return the same per-node capabilities, in the same order, as the
// always-heap `split`. The stack path's raw-pointer writes are the highest-
// risk unsafe surface in this module; only a topology crossing the 128-node
// boundary exercises both branches.

/// A synthetic `CpuTopology` of `node_count` single-processor nodes with
/// distinct ids `0..node_count`.
fn synthetic_topology(node_count: usize) -> CpuTopology {
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

fn node_ids_via_split(topology: &CpuTopology) -> Vec<u32> {
    sync_region_placement_scope(|placement| {
        placement
            .split(topology)
            .iter()
            .map(|p| p.node_id().get())
            .collect()
    })
}

fn node_ids_via_split_with(topology: &CpuTopology) -> Vec<u32> {
    sync_region_placement_scope(|placement| {
        placement.split_with(topology, |permits| {
            permits.iter().map(|p| p.node_id().get()).collect()
        })
    })
}

#[test]
fn split_with_matches_split_across_the_stack_heap_boundary() {
    // 1 (degenerate), well within the stack path, exactly at the boundary
    // (127/128), and just past it (129/140) into the heap fallback.
    for node_count in [1usize, 2, 64, 127, 128, 129, 140] {
        let topology = synthetic_topology(node_count);
        let via_split = node_ids_via_split(&topology);
        let via_split_with = node_ids_via_split_with(&topology);
        assert_eq!(
            via_split.len(),
            node_count,
            "split: wrong count at node_count={node_count}"
        );
        assert_eq!(
            via_split, via_split_with,
            "split vs split_with diverged at node_count={node_count}"
        );
    }
}

proptest::proptest! {
    #[test]
    fn split_with_matches_split_proptest(node_count in 1usize..=200) {
        let topology = synthetic_topology(node_count);
        let via_split = node_ids_via_split(&topology);
        let via_split_with = node_ids_via_split_with(&topology);
        proptest::prop_assert_eq!(via_split, via_split_with);
    }
}

// ── Placement-partition regression coverage ──
//
// `split_static` duplicates the brand's single Melinoe write token, so the
// NUMA node tag is the only thing keeping the resulting capabilities apart.
// That works because a cell answers to exactly one tag: the pinned wrappers
// derive their tag from ownership or from an exclusive borrow, never from a
// caller argument. The negative half of this proof is the `compile_fail`
// doctest on `SyncRegionPlacement::split_static`; these are the positive half.

#[test]
fn statically_split_capabilities_write_their_own_cells() {
    let (a, b) = sync_region_placement_scope(|region| {
        let cell_a = ConstNumaPinnedCell::<0, u32>::new(10);
        let cell_b = ConstNumaPinnedCell::<1, u32>::new(20);

        let (mut p0, mut p1) = region.split_static::<0, 1>();

        // Disjoint by construction: `cell_a` is only nameable as node 0.
        *p0.write(&cell_a) += 1;
        *p1.write(&cell_b) += 2;

        (*p0.read(&cell_a), *p1.read(&cell_b))
    });

    assert_eq!((a, b), (11, 22));
}

#[test]
fn pinned_references_inherit_the_owner_tag() {
    let observed = sync_region_placement_scope(|region| {
        let owned = ConstNumaPinnedCell::<3, u32>::new(5);
        let borrowed = owned.as_pinned_ref();

        let mut permit = region.project_static::<3>();
        *permit.write(&borrowed) *= 7;
        *permit.read(&owned)
    });

    assert_eq!(observed, 35);
}

#[test]
fn dynamic_pinned_reference_inherits_the_owner_tag() {
    let owned = NumaPinnedCell::new(NumaNodeId::new(2), 9u32);
    let borrowed = owned.as_pinned_ref();
    assert_eq!(borrowed.node_id(), NumaNodeId::new(2));
    assert_eq!(owned.node_id(), NumaNodeId::new(2));
}

#[test]
fn duplicate_node_ids_cannot_reach_a_capability_split() {
    // `split`'s tag-distinctness precondition is upheld one level earlier:
    // `CpuTopology` refuses to build a node table that repeats an id, so a
    // topology able to mint two same-tag capabilities cannot be constructed.
    // The split-local re-check of the same precondition is unit-tested in
    // `branded::region`.
    let nodes = vec![
        NumaNode {
            id: NumaNodeId::new(0),
            processors: vec![0].into_boxed_slice(),
            distances: vec![10, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        NumaNode {
            id: NumaNodeId::new(0),
            processors: vec![1].into_boxed_slice(),
            distances: vec![10, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let processor_to_node =
        build_processor_to_node(2, &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(0))]);

    let built = std::panic::catch_unwind(|| {
        CpuTopology::new_for_test(
            TopologyEpoch::INITIAL,
            nodes.into_boxed_slice(),
            processor_to_node,
            2,
            None,
        )
    });

    assert!(
        built.is_err(),
        "CpuTopology accepted a duplicate NUMA node id"
    );
}

#[test]
fn disjoint_halves_of_one_buffer_take_different_tags() {
    // The use case the fix must not break: one buffer split across two NUMA
    // nodes. `split_at_mut` proves the halves do not overlap, so each may take
    // its own tag — this is a real partition, unlike labelling one cell twice.
    let (left, right) = sync_region_placement_scope(|region| {
        let mut cells = [
            region.cell(1u32),
            region.cell(2u32),
            region.cell(3u32),
            region.cell(4u32),
        ];
        let (lo, hi) = cells.split_at_mut(2);

        let lo_pinned = ConstNumaPinnedSliceRef::<0, u32>::from_unique(lo);
        let hi_pinned = ConstNumaPinnedSliceRef::<1, u32>::from_unique(hi);

        let (mut p0, mut p1) = region.split_static::<0, 1>();

        for value in p0.write_slice(&lo_pinned) {
            *value *= 10;
        }
        for value in p1.write_slice(&hi_pinned) {
            *value *= 100;
        }

        (
            p0.read_slice(&lo_pinned).to_vec(),
            p1.read_slice(&hi_pinned).to_vec(),
        )
    });

    assert_eq!(left, vec![10, 20]);
    assert_eq!(right, vec![300, 400]);
}
