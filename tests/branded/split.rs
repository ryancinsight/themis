//! Branded split-entry-point unit tests.
//!
//! Test code is exempt from `clippy::unwrap_used`: a panic here is the
//! failure report, not a shipped panic path.

use themis::{
    build_processor_to_node, CpuTopology, MemoryTier, NumaNode, NumaNodeId, TopologyEpoch,
};
use themis::{
    sync_region_placement_scope, ConstNumaPinnedCell, ConstNumaPinnedCellRef,
    ConstNumaPinnedSliceRef, NumaPinnedCell, NumaPinnedCellRef, NumaPinnedSliceRef,
};

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

// â”€â”€ `split` vs `split_with` differential coverage â”€â”€
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
#[test]
fn split_with_matches_split_across_the_stack_heap_boundary() {
    // 1 (degenerate), well within the stack path, exactly at the boundary
    // (127/128), and just past it (129/140) into the heap fallback.
    for node_count in [1usize, 2, 64, 127, 128, 129, 140] {
        let topology = crate::support::synthetic_topology(node_count);
        let via_split = crate::support::node_ids_via_split(&topology);
        let via_split_with = crate::support::node_ids_via_split_with(&topology);
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
        let topology = crate::support::synthetic_topology(node_count);
        let via_split = crate::support::node_ids_via_split(&topology);
        let via_split_with = crate::support::node_ids_via_split_with(&topology);
        proptest::prop_assert_eq!(via_split, via_split_with);
    }
}

// â”€â”€ Placement-partition regression coverage â”€â”€
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
        built.map_or(true, |_| false),
        "CpuTopology accepted a duplicate NUMA node id"
    );
}

#[test]
fn disjoint_halves_of_one_buffer_take_different_tags() {
    // The use case the fix must not break: one buffer split across two NUMA
    // nodes. `split_at_mut` proves the halves do not overlap, so each may take
    // its own tag â€” this is a real partition, unlike labelling one cell twice.
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
