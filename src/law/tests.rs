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
    assert_eq!(TopologyEpoch::new(19).next().get(), 20);
    assert_eq!(TopologyEpoch::new(u64::MAX).next().get(), 0);
    assert!(NumaNodeId::new(7).is_valid());
    assert!(!NumaNodeId::INVALID.is_valid());
    assert!(WorkerId::new(3).is_valid());
    assert!(!WorkerId::INVALID.is_valid());
    assert!(LocalityDomainId::new(11).is_valid());
    assert!(!LocalityDomainId::INVALID.is_valid());
}


#[test]
fn default_placement_is_current_dram() {
    assert_eq!(PlacementHint::default(), PlacementHint::Current);
    assert_eq!(MemoryTier::default(), MemoryTier::Dram);
}
