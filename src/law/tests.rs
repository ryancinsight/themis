//! Placement law unit tests.

use super::{
    LocalityDomainId, MemoryTier, NumaBucketIndex, NumaNodeId, PlacementHint, TopologyEpoch,
    WorkerId,
};

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

// ── Property-based modular-arithmetic invariants ──
//
// `NumaBucketIndex<BUCKETS>::new`/`wrapping_add` were example-tested only at a
// single (raw, offset) pair. `BUCKETS` is a const generic, so proptest can't
// quantify over it directly; instead these fix representative table sizes
// (1 — degenerate, 7 — non-power-of-two, 16 — power-of-two) and quantify over
// arbitrary raw node values and wrap offsets, which is where off-by-one and
// overflow bugs in modular placement tables actually hide.

macro_rules! bucket_index_properties {
    ($mod_name:ident, $buckets:literal) => {
        mod $mod_name {
            // The BUCKETS=1 instantiation makes `% $buckets` literally `% 1`,
            // which clippy flags as always-zero; that degenerate case is the
            // exact boundary this suite exists to cover (a single-bucket
            // placement table must still report index 0 for any input), so the
            // "pointless" modulo is the assertion under test, not dead code.
            #![allow(clippy::modulo_one)]

            use super::NumaBucketIndex;

            proptest::proptest! {
                /// A freshly normalized index is always within the table.
                #[test]
                fn new_is_always_in_range(raw in 0u32..=u32::MAX) {
                    let idx = NumaBucketIndex::<$buckets>::new(raw as usize);
                    proptest::prop_assert!(idx.index() < $buckets);
                }

                /// `new` matches the plain `%` reference for any raw value.
                #[test]
                fn new_matches_modulo_reference(raw in 0usize..1_000_000) {
                    let idx = NumaBucketIndex::<$buckets>::new(raw);
                    proptest::prop_assert_eq!(idx.index(), raw % $buckets);
                }

                /// `wrapping_add` never leaves the table and matches the
                /// closed-form `(start + offset) % BUCKETS` for any start/offset.
                #[test]
                fn wrapping_add_matches_modulo_reference(
                    raw in 0usize..1_000_000,
                    offset in 0usize..1_000_000,
                ) {
                    let start = NumaBucketIndex::<$buckets>::new(raw);
                    let advanced = start.wrapping_add(offset);
                    proptest::prop_assert!(advanced.index() < $buckets);
                    proptest::prop_assert_eq!(advanced.index(), (start.index() + offset) % $buckets);
                }

                /// Advancing by the full table size is the identity (a full lap).
                #[test]
                fn wrapping_add_full_cycle_is_identity(raw in 0usize..1_000_000) {
                    let start = NumaBucketIndex::<$buckets>::new(raw);
                    proptest::prop_assert_eq!(start.wrapping_add($buckets).index(), start.index());
                }
            }
        }
    };
}

bucket_index_properties!(buckets_1, 1);
bucket_index_properties!(buckets_7, 7);
bucket_index_properties!(buckets_16, 16);
