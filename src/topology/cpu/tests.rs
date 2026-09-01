//! CPU efficiency-class value and detection tests.

use super::{CpuTopology, EfficiencyClass};

/// The performance-core mask of the developer host whose mislabelling
/// motivated this capability: `{0, 1, 10, 11, 12, 13, 22, 23}` out of 24
/// logical processors. Deliberately not a contiguous low range.
const PERFORMANCE_MASK: u64 = 0xc0_3c03;

fn hybrid_host() -> CpuTopology {
    let classes: Box<[EfficiencyClass]> = (0..24u32)
        .map(|processor| {
            EfficiencyClass::new(u8::from(PERFORMANCE_MASK & (1u64 << processor) != 0))
        })
        .collect();
    CpuTopology::single_node(24).with_efficiency_classes_for_test(Some(classes))
}

fn homogeneous_host() -> CpuTopology {
    let classes: Box<[EfficiencyClass]> = vec![EfficiencyClass::LOWEST; 16].into();
    CpuTopology::single_node(16).with_efficiency_classes_for_test(Some(classes))
}

#[test]
fn the_naive_class_comparison_fabricates_a_performance_core() {
    let topology = CpuTopology::single_node(8);

    // The trap this predicate exists for: both sides are `None`, so `==`
    // answers "yes" for every processor, including one outside the
    // snapshot. Asserting the defective spelling still misbehaves keeps
    // this test from passing vacuously if absence semantics ever change.
    assert!(
        topology.processor_efficiency_class(999) == topology.highest_efficiency_class(),
        "the fabricating comparison no longer fabricates; this oracle is stale",
    );

    // The predicate disagrees with it, which is the entire point.
    assert_eq!(topology.is_in_highest_class(0), None);
    assert_eq!(topology.is_in_highest_class(999), None);
}

#[test]
fn the_predicate_separates_performance_from_efficiency_cores() {
    let topology = hybrid_host();

    for processor in 0..24u32 {
        let expected = PERFORMANCE_MASK & (1u64 << processor) != 0;
        assert_eq!(
            topology.is_in_highest_class(processor),
            Some(expected),
            "processor {processor} misclassified",
        );
    }

    // The specific inversion that motivated the capability: cpu 2 reads as
    // a performance core to the naive comparison on an unreporting host,
    // and is an efficiency core here.
    assert_eq!(topology.is_in_highest_class(2), Some(false));
    assert_eq!(topology.is_in_highest_class(1), Some(true));
}

#[test]
fn a_processor_outside_the_snapshot_is_absent_not_false() {
    let topology = hybrid_host();
    assert_eq!(topology.is_in_highest_class(24), None);
    assert_eq!(topology.is_in_highest_class(u32::MAX), None);
}

#[test]
fn every_processor_of_a_homogeneous_host_is_in_its_highest_class() {
    let topology = homogeneous_host();

    for processor in 0..16u32 {
        assert_eq!(topology.is_in_highest_class(processor), Some(true));
    }

    // Reported-and-uniform stays distinguishable from unreported.
    assert_eq!(topology.efficiency_class_count(), Some(1));
    assert_eq!(topology.is_in_highest_class(16), None);
}

#[test]
fn a_performance_processor_is_never_the_hardcoded_low_id() {
    let topology = hybrid_host();
    let fastest = topology
        .highest_efficiency_class()
        .expect("the hybrid fixture reports classes");
    let performance: Vec<u32> = topology
        .processors_in_efficiency_class(fastest)
        .expect("the hybrid fixture reports classes")
        .collect();

    assert_eq!(performance, vec![0, 1, 10, 11, 12, 13, 22, 23]);
    assert!(
        !performance.contains(&2),
        "cpu 2 is an efficiency core on this host; the retired hand-rolled \
         constant pinned probes to it"
    );
    assert!(
        topology
            .processor_efficiency_class(2)
            .expect("the hybrid fixture reports classes")
            < fastest
    );
}

#[test]
fn a_hybrid_host_reports_its_tier_count() {
    let topology = hybrid_host();
    assert_eq!(topology.efficiency_class_count(), Some(2));
    assert_eq!(topology.is_hybrid(), Some(true));
    assert_eq!(
        topology.highest_efficiency_class(),
        Some(EfficiencyClass::new(1))
    );
}

#[test]
fn a_homogeneous_host_is_one_class_and_not_absence() {
    let topology = homogeneous_host();
    assert_eq!(topology.efficiency_class_count(), Some(1));
    assert_eq!(topology.is_hybrid(), Some(false));
    assert_eq!(
        topology.highest_efficiency_class(),
        Some(EfficiencyClass::LOWEST)
    );
    assert_eq!(
        topology
            .processors_in_efficiency_class(EfficiencyClass::LOWEST)
            .expect("the homogeneous fixture reports classes")
            .count(),
        16
    );
    assert_eq!(topology.efficiency_classes().map(<[_]>::len), Some(16));
}

#[test]
fn an_unreported_host_is_absent_everywhere_not_homogeneous() {
    let topology = CpuTopology::single_node(8);
    assert_eq!(topology.efficiency_classes(), None);
    assert_eq!(topology.efficiency_class_count(), None);
    assert_eq!(topology.is_hybrid(), None);
    assert_eq!(topology.highest_efficiency_class(), None);
    assert_eq!(topology.processor_efficiency_class(0), None);
    assert!(topology
        .processors_in_efficiency_class(EfficiencyClass::LOWEST)
        .is_none());
}

#[test]
fn a_class_outside_the_reported_range_selects_no_processor() {
    let topology = hybrid_host();
    assert_eq!(
        topology
            .processors_in_efficiency_class(EfficiencyClass::new(9))
            .expect("the hybrid fixture reports classes")
            .count(),
        0
    );
}

#[test]
fn processors_outside_the_snapshot_have_no_class() {
    let topology = hybrid_host();
    assert_eq!(topology.processor_efficiency_class(24), None);
    assert_eq!(topology.processor_efficiency_class(u32::MAX), None);
}

/// Host-dependent smoke test over whatever this machine reports. It skips
/// with a printed reason rather than passing silently, so a homogeneous or
/// unreporting host never masquerades as coverage of the parsers.
#[test]
#[expect(
    clippy::print_stdout,
    reason = "a skipped host-dependent test must say why it skipped"
)]
fn detected_efficiency_classes_satisfy_their_invariants() {
    let topology = CpuTopology::detect().expect("detection returns at least a fallback");
    let Some(classes) = topology.efficiency_classes() else {
        println!(
            "SKIP detected_efficiency_classes_satisfy_their_invariants: this host \
             reports no efficiency classes for all {} logical processors; the \
             parsers are covered by the recorded fixtures instead",
            topology.logical_processors()
        );
        return;
    };

    assert_eq!(classes.len(), topology.logical_processors());

    let mut distinct: Vec<u8> = classes.iter().copied().map(EfficiencyClass::rank).collect();
    distinct.sort_unstable();
    distinct.dedup();
    let dense: Vec<u8> = (0..u8::try_from(distinct.len()).unwrap_or(u8::MAX)).collect();
    assert_eq!(distinct, dense, "ranks must be dense");
    assert_eq!(topology.efficiency_class_count(), Some(distinct.len()));
    assert_eq!(topology.is_hybrid(), Some(distinct.len() > 1));

    let fastest = topology
        .highest_efficiency_class()
        .expect("a reported table has a highest class");
    let performance: Vec<u32> = topology
        .processors_in_efficiency_class(fastest)
        .expect("a reported table yields an iterator")
        .collect();
    assert!(
        !performance.is_empty(),
        "dense ranks guarantee the highest class is populated"
    );
    for processor in &performance {
        assert_eq!(
            topology.processor_efficiency_class(*processor),
            Some(fastest)
        );
    }
    println!(
        "host reports {} efficiency class(es) over {} processors; class {} holds {:?}",
        distinct.len(),
        topology.logical_processors(),
        fastest.rank(),
        performance
    );
}
