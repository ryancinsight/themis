//! Branded placement scope unit tests.

use super::{sync_region_placement_scope, thread_local_placement_scope};
use crate::{MemoryTier, NumaNodeId, PlacementHint};

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
    let nodes = std::vec![
        crate::NumaNode {
            id: NumaNodeId::new(0),
            processors: std::vec![0].into_boxed_slice(),
            distances: std::vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        crate::NumaNode {
            id: NumaNodeId::new(1),
            processors: std::vec![1].into_boxed_slice(),
            distances: std::vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let topology = crate::CpuTopology {
        epoch: crate::TopologyEpoch::INITIAL,
        processor_to_node: crate::topology::build_processor_to_node(
            2,
            &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))],
        ),
        node_to_index: crate::topology::build_node_to_index(&nodes),
        adjacent_nodes: crate::topology::build_adjacent_nodes(&nodes),
        numa_nodes: nodes.into_boxed_slice(),
        logical_processors: 2,
        cache_levels: crate::topology::default_cache_levels(2),
    };

    let (val0, val1) = sync_region_placement_scope(|placement| {
        let cell0 = super::NumaPinnedCell::new(NumaNodeId::new(0), 100u32);
        let cell1 = super::NumaPinnedCell::new(NumaNodeId::new(1), 200u32);

        let mut permits = placement.split(&topology);
        assert_eq!(permits.len(), 2);

        let mut permit0 = permits.remove(0);
        let mut permit1 = permits.remove(0);

        *permit0.write(&cell0).unwrap() = 111u32;
        *permit1.write(&cell1).unwrap() = 222u32;

        assert!(permit0.write(&cell1).is_none());
        assert!(permit1.write(&cell0).is_none());

        (*permit0.read(&cell0).unwrap(), *permit1.read(&cell1).unwrap())
    });

    assert_eq!(val0, 111);
    assert_eq!(val1, 222);
}

#[test]
fn sync_region_split_with_avoid_heap_allocations() {
    let nodes = std::vec![
        crate::NumaNode {
            id: NumaNodeId::new(0),
            processors: std::vec![0].into_boxed_slice(),
            distances: std::vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        crate::NumaNode {
            id: NumaNodeId::new(1),
            processors: std::vec![1].into_boxed_slice(),
            distances: std::vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let topology = crate::CpuTopology {
        epoch: crate::TopologyEpoch::INITIAL,
        processor_to_node: crate::topology::build_processor_to_node(
            2,
            &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))],
        ),
        node_to_index: crate::topology::build_node_to_index(&nodes),
        adjacent_nodes: crate::topology::build_adjacent_nodes(&nodes),
        numa_nodes: nodes.into_boxed_slice(),
        logical_processors: 2,
        cache_levels: crate::topology::default_cache_levels(2),
    };

    let (val0, val1) = sync_region_placement_scope(|placement| {
        let cell0 = super::NumaPinnedCell::new(NumaNodeId::new(0), 500u32);
        let cell1 = super::NumaPinnedCell::new(NumaNodeId::new(1), 600u32);

        placement.split_with(&topology, |permits| {
            assert_eq!(permits.len(), 2);
            let mut iter = permits.iter_mut();
            let permit0 = iter.next().unwrap();
            let permit1 = iter.next().unwrap();

            *permit0.write(&cell0).unwrap() = 555u32;
            *permit1.write(&cell1).unwrap() = 666u32;

            assert!(permit0.write(&cell1).is_none());
            assert!(permit1.write(&cell0).is_none());

            (*permit0.read(&cell0).unwrap(), *permit1.read(&cell1).unwrap())
        })
    });

    assert_eq!(val0, 555);
    assert_eq!(val1, 666);
}

#[test]
fn sync_region_pinned_slice_allows_efficient_bulk_access() {
    let nodes = std::vec![
        crate::NumaNode {
            id: NumaNodeId::new(0),
            processors: std::vec![0].into_boxed_slice(),
            distances: std::vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        crate::NumaNode {
            id: NumaNodeId::new(1),
            processors: std::vec![1].into_boxed_slice(),
            distances: std::vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let topology = crate::CpuTopology {
        epoch: crate::TopologyEpoch::INITIAL,
        processor_to_node: crate::topology::build_processor_to_node(
            2,
            &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))],
        ),
        node_to_index: crate::topology::build_node_to_index(&nodes),
        adjacent_nodes: crate::topology::build_adjacent_nodes(&nodes),
        numa_nodes: nodes.into_boxed_slice(),
        logical_processors: 2,
        cache_levels: crate::topology::default_cache_levels(2),
    };

    let (s0_sum, s1_sum) = sync_region_placement_scope(|placement| {
        let slice0 = super::NumaPinnedSlice::new(NumaNodeId::new(0), std::vec![1, 2, 3]);
        let slice1 = super::NumaPinnedSlice::new(NumaNodeId::new(1), std::vec![10, 20, 30]);

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

        let sum0 = permit0.read_slice(&slice0).map(|s| s.iter().sum::<i32>()).unwrap();
        let sum1 = permit1.read_slice(&slice1).map(|s| s.iter().sum::<i32>()).unwrap();
        (sum0, sum1)
    });

    assert_eq!(s0_sum, 12); // (1*2 + 2*2 + 3*2) = 12
    assert_eq!(s1_sum, 75); // (15 + 25 + 35) = 75
}

#[test]
fn const_numa_branding_provides_zero_cost_static_access() {
    let (val0, val1) = sync_region_placement_scope(|placement| {
        let cell0 = super::ConstNumaPinnedCell::<0, u32>::new(700);
        let cell1 = super::ConstNumaPinnedCell::<1, u32>::new(800);

        let (mut permit0, mut permit1) = placement.split_static::<0, 1>();

        *permit0.write(&cell0) = 777;
        *permit1.write(&cell1) = 888;

        (*permit0.read(&cell0), *permit1.read(&cell1))
    });

    assert_eq!(val0, 777);
    assert_eq!(val1, 888);
}

#[test]
fn const_numa_pinned_slices_support_direct_borrowing() {
    let sum = sync_region_placement_scope(|placement| {
        let slice = super::ConstNumaPinnedSlice::<0, u32>::new(std::vec![10, 20, 30]);
        let mut permit = unsafe { placement.project_static::<0>() };

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
        let cell0 = super::NumaPinnedCell::new(NumaNodeId::new(0), 10u32);
        let cell1 = super::NumaPinnedCell::new(NumaNodeId::new(1), 20u32);

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
        let cell0 = super::ConstNumaPinnedCell::<0, u32>::new(100);
        let mut permit = placement.pin_local_static::<0>();
        *permit.write(&cell0) += 50;
        (*permit.read(&cell0), 0)
    });

    let sum1_actual = thread_local_placement_scope(|placement| {
        let slice1 = super::ConstNumaPinnedSlice::<1, u32>::new(std::vec![1, 2, 3]);
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
    use super::{NumaPinnedCellRef, NumaPinnedSliceRef};
    use melinoe::MelinoeCell;

    let nodes = std::vec![
        crate::NumaNode {
            id: NumaNodeId::new(0),
            processors: std::vec![0].into_boxed_slice(),
            distances: std::vec![10, 20].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
        crate::NumaNode {
            id: NumaNodeId::new(1),
            processors: std::vec![1].into_boxed_slice(),
            distances: std::vec![20, 10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        },
    ];
    let topology = crate::CpuTopology {
        epoch: crate::TopologyEpoch::INITIAL,
        processor_to_node: crate::topology::build_processor_to_node(
            2,
            &[(0, NumaNodeId::new(0)), (1, NumaNodeId::new(1))],
        ),
        node_to_index: crate::topology::build_node_to_index(&nodes),
        adjacent_nodes: crate::topology::build_adjacent_nodes(&nodes),
        numa_nodes: nodes.into_boxed_slice(),
        logical_processors: 2,
        cache_levels: crate::topology::default_cache_levels(2),
    };

    let (val, sum) = sync_region_placement_scope(|placement| {
        // Stack-allocated cells and arrays
        let cell_raw = MelinoeCell::new(42u32);
        let cell_ref = NumaPinnedCellRef::new(NumaNodeId::new(0), &cell_raw);

        let array_raw = [
            MelinoeCell::new(10u32),
            MelinoeCell::new(20u32),
            MelinoeCell::new(30u32),
        ];
        let slice_ref = NumaPinnedSliceRef::new(NumaNodeId::new(0), &array_raw);

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
    use super::{ConstNumaPinnedCellRef, ConstNumaPinnedSliceRef};
    use melinoe::MelinoeCell;

    let (val, sum) = sync_region_placement_scope(|placement| {
        let cell_raw = MelinoeCell::new(100u32);
        let cell_ref = ConstNumaPinnedCellRef::<5, u32>::new(&cell_raw);

        let array_raw = [
            MelinoeCell::new(1u32),
            MelinoeCell::new(2u32),
        ];
        let slice_ref = ConstNumaPinnedSliceRef::<5, u32>::new(&array_raw);

        let mut permit = unsafe { placement.project_static::<5>() };

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
