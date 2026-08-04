//! Placement hints, NUMA bucket arithmetic, and memory-tier classification.
//!
//! Themis owns placement vocabulary, not allocation.  This example
//! constructs [`PlacementHint`] values, matches on them, maps NUMA node IDs
//! into a fixed-size bucket table, and queries the [`MemoryTier`] host-
//! allocatability contract.

use themis::{
    LocalityDomainId, MemoryTier, NumaNodeId, PlacementHint, WorkerId,
};

/// Simulate how an allocator would inspect a placement hint.
fn describe_hint(hint: PlacementHint) -> &'static str {
    match hint {
        PlacementHint::Current         => "allocate on the caller's current NUMA node",
        PlacementHint::Numa(_)         => "allocate on the specified NUMA node",
        PlacementHint::Domain(_)       => "allocate within the specified locality domain",
        PlacementHint::Tier(tier) if tier.is_host_allocatable()
                                       => "allocate from the specified host-allocatable tier",
        PlacementHint::Tier(_)         => "invalid for allocation — budgeted tier only",
        PlacementHint::Any             => "allocate anywhere",
    }
}

fn main() {
    // Default hint is Current.
    assert_eq!(PlacementHint::default(), PlacementHint::Current);
    println!("default: {}", describe_hint(PlacementHint::default()));

    // Explicit NUMA-node preference.
    let node2 = NumaNodeId::new(2);
    println!("NUMA({node2:?}): {}", describe_hint(PlacementHint::Numa(node2)));

    // Locality-domain preference.
    let domain0 = LocalityDomainId::new(0);
    println!("Domain({domain0:?}): {}", describe_hint(PlacementHint::Domain(domain0)));

    // Tier preference — HBM is host-allocatable.
    let hbm = PlacementHint::Tier(MemoryTier::Hbm);
    println!("Tier(Hbm): {}", describe_hint(hbm));

    // Registers is NOT host-allocatable (GPU compiler-assigned budget).
    let regs = PlacementHint::Tier(MemoryTier::Registers);
    println!("Tier(Registers): {}", describe_hint(regs));
    assert!(!MemoryTier::Registers.is_host_allocatable());
    assert!(!MemoryTier::SharedMem.is_host_allocatable());

    // Enumerate all host-allocatable tiers.
    let allocatable: Vec<_> = [
        MemoryTier::Dram, MemoryTier::Hbm, MemoryTier::Gddr,
        MemoryTier::HostPinned, MemoryTier::Device, MemoryTier::Persistent,
        MemoryTier::Registers, MemoryTier::SharedMem,
    ]
    .iter()
    .copied()
    .filter(|t| t.is_host_allocatable())
    .collect();
    println!("host-allocatable tiers: {allocatable:?}");
    assert_eq!(allocatable.len(), 6);

    // NumaBucketIndex: map node IDs into a 4-bucket table (wrapping).
    const BUCKETS: usize = 4;
    for raw in 0u32..8 {
        let node = NumaNodeId::new(raw);
        let bucket = node.bucket_index::<BUCKETS>();
        println!("node {raw} → bucket {}", bucket.index());
        assert_eq!(bucket.index(), raw as usize % BUCKETS);
    }

    // wrapping_add stays within the bucket count.
    let b2 = NumaNodeId::new(2).bucket_index::<BUCKETS>();
    assert_eq!(b2.wrapping_add(3).index(), 1); // (2 + 3) % 4 = 1

    // INVALID sentinels.
    assert!(!NumaNodeId::INVALID.is_valid());
    assert!(!WorkerId::INVALID.is_valid());
    println!("INVALID nodes and workers correctly report is_valid() = false");
}
