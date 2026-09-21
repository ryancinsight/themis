//! Branded placement scope unit tests.
//!
//! Test code is exempt from `clippy::unwrap_used`: a panic here is the
//! failure report, not a shipped panic path.

use themis::{
    build_processor_to_node, sync_region_placement_scope, thread_local_placement_scope,
    ConstNumaPinnedSlice, CpuTopology, MemoryTier, NumaNode, NumaNodeId, NumaPinnedCell,
    NumaPinnedSlice, PlacementHint, TopologyEpoch,
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
        let topology = crate::support::synthetic_topology(1);
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
        let topology = crate::support::synthetic_topology(1);
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

/// Themis delegates execution rather than owning it, so its placement
/// partition API must reach whatever executor is registered process-wide.
///
/// This pins that seam. A counting executor is registered, the placement API
/// is driven, and the test asserts the executor was actually entered with the
/// expected shard count. Without this, a regression that bypassed the
/// registered executor (and silently reverted to per-call OS-thread creation)
/// would be invisible: results would still be correct, only far slower.
#[test]
fn placement_partition_routes_through_the_registered_executor() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static LAST_TASKS: AtomicUsize = AtomicUsize::new(0);

    /// A stand-in scheduler that runs tasks in ascending index order on the
    /// calling thread, counting how many calls it received.
    struct Counting;

    // SAFETY: `run_indexed` drives every index in `0..num_tasks` exactly once
    // and does not return until the last has finished, which is the
    // `ParallelExecutor` contract. Running on the calling thread adds no
    // concurrency, so the context pointer is never observed elsewhere.
    unsafe impl melinoe::sync::ParallelExecutor for Counting {
        unsafe fn run_indexed(num_tasks: usize, task: unsafe fn(usize, *mut ()), context: *mut ()) {
            CALLS.fetch_add(1, Ordering::SeqCst);
            LAST_TASKS.store(num_tasks, Ordering::SeqCst);
            for index in 0..num_tasks {
                // SAFETY: forwarded from the caller; this implementation invokes
                // each index exactly once with the caller's context.
                unsafe { task(index, context) };
            }
        }
    }

    melinoe::sync::register_parallel_executor::<Counting>();

    sync_region_placement_scope(|placement| {
        let mut values = NumaPinnedSlice::from_fn(NumaNodeId::new(0), 12, |index| index);
        let topology = crate::support::synthetic_topology(1);
        let mut permits = placement.split(&topology);
        let mut permit = permits.pop().expect("synthetic topology has one node");

        // 12 elements in chunks of 3 -> exactly 4 shards.
        let plan = melinoe::sync::PartitionPlan::chunk_size(3);
        permit
            .partition_for_each_mut_with(&mut values, plan, |start, shard| {
                for (offset, value) in shard.iter_mut().enumerate() {
                    *value = start + offset;
                }
            })
            .expect("matching dynamic node permit");
    });

    let calls = CALLS.load(Ordering::SeqCst);
    let tasks = LAST_TASKS.load(Ordering::SeqCst);
    // Clean up before asserting so a failure cannot leak the executor into
    // other tests in this binary.
    melinoe::sync::clear_parallel_executor();

    assert_eq!(
        calls, 1,
        "the registered executor must be entered exactly once"
    );
    assert_eq!(tasks, 4, "12 elements at chunk size 3 is four shards");
}
