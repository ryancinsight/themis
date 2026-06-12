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
