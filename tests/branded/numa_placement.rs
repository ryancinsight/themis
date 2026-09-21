//! Static and thread-local NUMA placement tests.
//!
//! The const-branded surface (`ConstNumaPinnedCell`, `ConstNumaPinnedSlice`,
//! `split_static`) carries its node in the type, so these assert what the
//! dynamic scope tests cannot: that the node is a compile-time fact and the
//! permits it yields are disjoint.

use themis::{
    sync_region_placement_scope, thread_local_placement_scope, ConstNumaPinnedCell,
    ConstNumaPinnedSlice, NumaNodeId, NumaPinnedCell,
};

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
