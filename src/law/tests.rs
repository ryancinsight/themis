//! Placement law unit tests.

use super::{LocalityDomainId, MemoryTier, NumaNodeId, PlacementHint, TopologyEpoch, WorkerId};

#[test]
fn typed_ids_preserve_values() {
    assert_eq!(NumaNodeId::new(7).get(), 7);
    assert_eq!(NumaNodeId::new(7).index(), 7);
    assert_eq!(NumaNodeId::new(19).bucket_index::<16>().index(), 3);
    assert_eq!(
        NumaNodeId::new(19)
            .bucket_index::<16>()
            .wrapping_add(15)
            .index(),
        2
    );
    assert_eq!(WorkerId::new(3).get(), 3);
    assert_eq!(WorkerId::new(3).index(), 3);
    assert_eq!(LocalityDomainId::new(11).get(), 11);
    assert_eq!(TopologyEpoch::new(19).get(), 19);
}

#[test]
#[should_panic(expected = "NUMA bucket count must be non-zero")]
fn zero_bucket_count_uses_domain_panic() {
    let _ = NumaNodeId::new(19).bucket_index::<0>();
}

#[test]
fn default_placement_is_current_dram() {
    assert_eq!(PlacementHint::default(), PlacementHint::Current);
    assert_eq!(MemoryTier::default(), MemoryTier::Dram);
}
