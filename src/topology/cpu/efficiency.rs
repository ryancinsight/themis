//! Core efficiency-class discovery.
//!
//! A hybrid CPU mixes performance and efficient cores, and the mix is not
//! inferable from core counts, processor ids, or model strings — the developer
//! host that motivated this module reports its performance cores as the
//! non-contiguous mask `0xc03c03`, so "the low ids are the fast ones" is wrong
//! there by exactly the amount that matters. Only the platform knows, and a
//! platform that does not say is typed absence, never a fabricated split.

// The pure parsers are compiled wherever their fixtures can run — in the test
// build of either backend target, and on the target that actually reads them —
// so the Linux CI leg verifies the Windows record walk and vice versa.
// Compiling a parser its target never calls would be dead code under the lint
// floor.
mod rank;
#[cfg(any(test, windows))]
pub(in crate::topology::cpu) mod records;
#[cfg(any(test, target_os = "linux"))]
mod sysfs;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
pub(in crate::topology::cpu) mod windows;

use super::MAX_PROCESSOR_ID;
use crate::topology::types::EfficiencyClass;

/// Detects the per-processor efficiency-class table for a snapshot.
///
/// The table is indexed by processor id and covers exactly
/// `0..logical_processors`. `None` means the platform reported nothing usable;
/// `Some` with one distinct class means a homogeneous host.
///
/// The module is compiled only on a target with a backend, so there is no
/// third branch here; a target with neither never reaches this call.
pub(crate) fn detect_efficiency_classes(
    logical_processors: usize,
) -> Option<Box<[EfficiencyClass]>> {
    #[cfg(target_os = "linux")]
    {
        linux::detect(logical_processors)
    }

    #[cfg(windows)]
    {
        windows::detect(logical_processors)
    }
}

/// Ranks raw per-processor capability values into a class table.
///
/// `raw` must already cover exactly `0..logical_processors`; the length check
/// here is the last gate before the table becomes public data.
fn classes_from_capacities(
    raw: &[u32],
    logical_processors: usize,
) -> Option<Box<[EfficiencyClass]>> {
    if logical_processors == 0
        || logical_processors > MAX_PROCESSOR_ID
        || raw.len() != logical_processors
    {
        return None;
    }
    rank::dense_ranks(raw)
}

/// Assembles Windows `RelationProcessorCore` records into a class table.
///
/// Separated from the call that produces `bytes` so the whole path from
/// platform buffer to class table is testable on any target.
///
/// Returns `None` when the walk fails, when two records disagree about one
/// processor's class, or when the records do not cover exactly
/// `0..logical_processors`. Incomplete coverage is the multi-group case: this
/// crate numbers processors `group * 64 + bit`, and the Windows NUMA backend
/// that establishes `logical_processors` reads a single `GROUP_AFFINITY` per
/// node, so on a host whose groups the two APIs enumerate differently the class
/// table would be keyed by ids the rest of the snapshot does not use. Absence
/// there is the wrong answer withheld.
#[cfg(any(test, windows))]
fn classes_from_processor_records(
    bytes: &[u8],
    logical_processors: usize,
) -> Option<Box<[EfficiencyClass]>> {
    if logical_processors == 0 || logical_processors > MAX_PROCESSOR_ID {
        return None;
    }
    let entries = records::parse_processor_cores(bytes)?;
    let mut raw: Vec<Option<u8>> = vec![None; logical_processors];
    for entry in entries {
        let slot = raw.get_mut(usize::try_from(entry.processor).ok()?)?;
        match slot {
            Some(existing) if *existing != entry.raw_class => return None,
            _ => *slot = Some(entry.raw_class),
        }
    }
    let raw: Vec<u32> = raw
        .into_iter()
        .map(|class| class.map(u32::from))
        .collect::<Option<Vec<u32>>>()?;
    classes_from_capacities(&raw, logical_processors)
}

#[cfg(test)]
mod tests {
    use super::records::fixtures::{
        core_record, homogeneous_host_buffer, hybrid_host_buffer, PERFORMANCE_MASK,
    };
    use super::{classes_from_capacities, classes_from_processor_records};
    use crate::topology::types::EfficiencyClass;

    fn ranks(table: &[EfficiencyClass]) -> Vec<u8> {
        table.iter().map(|class| class.rank()).collect()
    }

    #[test]
    fn the_recorded_hybrid_host_ranks_its_performance_mask_highest() {
        let table = classes_from_processor_records(&hybrid_host_buffer(), 24)
            .expect("the recorded buffer covers all 24 processors");
        assert_eq!(table.len(), 24);

        let highest = *table.iter().max().expect("the table is not empty");
        let performance: Vec<u32> = table
            .iter()
            .enumerate()
            .filter(|(_, class)| **class == highest)
            .filter_map(|(processor, _)| u32::try_from(processor).ok())
            .collect();
        assert_eq!(performance, vec![0, 1, 10, 11, 12, 13, 22, 23]);

        let mask = performance
            .iter()
            .fold(0u64, |mask, processor| mask | (1u64 << processor));
        assert_eq!(mask, PERFORMANCE_MASK);
        assert!(
            table[2] < highest,
            "processor 2 is an efficiency core on this host, not a performance core"
        );
    }

    #[test]
    fn a_homogeneous_host_reports_one_class_and_is_not_absent() {
        let table = classes_from_processor_records(&homogeneous_host_buffer(), 16)
            .expect("the homogeneous buffer covers all 16 processors");
        assert_eq!(ranks(&table), vec![0; 16]);
    }

    #[test]
    fn sparse_platform_class_bytes_compress_to_dense_ranks() {
        // Windows may report classes 0 and 2 with nothing at 1.
        let bytes = [
            core_record(0, &[(0, 0b0011)]),
            core_record(2, &[(0, 0b1100)]),
        ]
        .concat();
        let table =
            classes_from_processor_records(&bytes, 4).expect("the buffer covers all 4 processors");
        assert_eq!(ranks(&table), vec![0, 0, 1, 1]);
    }

    #[test]
    fn incomplete_coverage_is_absent_rather_than_a_partial_table() {
        // Records cover processors 0..2 but the snapshot has 4.
        let bytes = core_record(1, &[(0, 0b0011)]);
        assert_eq!(classes_from_processor_records(&bytes, 4), None);
    }

    #[test]
    fn processors_outside_the_snapshot_are_absent() {
        // The multi-group case: the record enumeration reaches group 1 while
        // the snapshot only counts a single group's worth of processors.
        let bytes = [
            core_record(1, &[(0, u64::MAX)]),
            core_record(0, &[(1, 0b1)]),
        ]
        .concat();
        assert_eq!(classes_from_processor_records(&bytes, 64), None);
    }

    #[test]
    fn a_fully_enumerated_multi_group_host_is_reported() {
        let bytes = [
            core_record(1, &[(0, u64::MAX)]),
            core_record(0, &[(1, 0b11)]),
        ]
        .concat();
        let table = classes_from_processor_records(&bytes, 66)
            .expect("the records cover both groups completely");
        assert_eq!(table.len(), 66);
        assert_eq!(table[0].rank(), 1);
        assert_eq!(table[64].rank(), 0);
        assert_eq!(table[65].rank(), 0);
    }

    #[test]
    fn records_disagreeing_about_one_processor_are_absent() {
        let bytes = [
            core_record(0, &[(0, 0b0011)]),
            core_record(1, &[(0, 0b0110)]),
        ]
        .concat();
        assert_eq!(classes_from_processor_records(&bytes, 4), None);
    }

    #[test]
    fn empty_and_oversized_snapshots_are_absent() {
        assert_eq!(
            classes_from_processor_records(&hybrid_host_buffer(), 0),
            None
        );
        assert_eq!(classes_from_capacities(&[], 0), None);
        assert_eq!(classes_from_capacities(&[1, 2], 3), None);
        assert_eq!(classes_from_capacities(&[1, 2, 3], 2), None);
    }
}
