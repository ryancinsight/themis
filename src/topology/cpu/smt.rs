//! SMT sibling discovery: which logical processors share a physical core.
//!
//! Windows reports it in the same `RelationProcessorCore` records the
//! efficiency axis walks: every processor in one record is a thread of that
//! core. Linux reports it per processor at
//! `/sys/devices/system/cpu/cpuN/topology/thread_siblings_list`.
//!
//! Both reduce to one core ordinal per processor, then to dense [`CoreId`]s
//! assigned in ascending order of each core's lowest processor. The assembly
//! is pure and compiled on every target so its fixtures run in every CI leg;
//! only the reads that produce the platform data are gated. A host that reports
//! nothing usable is `None`, never a guessed "one thread per core".

use super::MAX_PROCESSOR_ID;
use crate::topology::types::CoreId;

/// Detects the per-processor core table for `0..logical_processors`.
///
/// `None` means the platform reported nothing usable, or reported a table that
/// does not cover exactly `0..logical_processors`. A host without SMT is a
/// present table with one processor per core, not absence.
pub(crate) fn detect_core_ids(logical_processors: usize) -> Option<Box<[CoreId]>> {
    #[cfg(target_os = "linux")]
    {
        linux::detect(logical_processors)
    }

    #[cfg(windows)]
    {
        cores_from_processor_records(
            &super::efficiency::windows::core_records()?,
            logical_processors,
        )
    }
}

/// Assembles Windows `RelationProcessorCore` records into a core table.
///
/// Every processor named by one record shares that record's core. Returns
/// `None` when the walk fails, when a processor appears in two records, or when
/// the records do not cover exactly `0..logical_processors`: the same coverage
/// rule the efficiency axis applies, for the same reason. A partial table would
/// fabricate "distinct cores" over the processors it missed.
#[cfg(any(test, windows))]
fn cores_from_processor_records(bytes: &[u8], logical_processors: usize) -> Option<Box<[CoreId]>> {
    if logical_processors == 0 || logical_processors > MAX_PROCESSOR_ID {
        return None;
    }
    let entries = super::efficiency::records::parse_processor_cores(bytes)?;
    let mut raw: Vec<Option<u32>> = vec![None; logical_processors];
    for entry in entries {
        let slot = raw.get_mut(usize::try_from(entry.processor).ok()?)?;
        match slot {
            Some(existing) if *existing != entry.core => return None,
            _ => *slot = Some(entry.core),
        }
    }
    let raw = raw.into_iter().collect::<Option<Vec<u32>>>()?;
    dense_core_ids(&raw)
}

/// Assembles per-processor `thread_siblings_list` contents into a core table.
///
/// `lists[n]` is the contents for processor `n`, or `None` when the file is
/// absent. Each list must be symmetric (if `m` is in the list of `n` then `n`
/// is in the list of `m`) and name only processors below `logical_processors`;
/// anything else is absence rather than a partially believed partition.
#[cfg(all(feature = "std", any(target_os = "linux", all(test, windows))))]
fn cores_from_sibling_lists(
    lists: &[Option<&str>],
    logical_processors: usize,
) -> Option<Box<[CoreId]>> {
    if logical_processors == 0
        || logical_processors > MAX_PROCESSOR_ID
        || lists.len() != logical_processors
    {
        return None;
    }
    let parsed: Vec<Vec<u32>> = lists
        .iter()
        .map(|list| list.map(super::parse_cpu_list))
        .collect::<Option<Vec<_>>>()?;
    let mut raw = Vec::with_capacity(logical_processors);
    for (processor, siblings) in parsed.iter().enumerate() {
        let processor = u32::try_from(processor).ok()?;
        if siblings.is_empty() || !siblings.contains(&processor) {
            return None;
        }
        let mut lowest = processor;
        for &sibling in siblings {
            let sibling_list = parsed.get(usize::try_from(sibling).ok()?)?;
            if !sibling_list.contains(&processor) {
                return None;
            }
            lowest = lowest.min(sibling);
        }
        raw.push(lowest);
    }
    dense_core_ids(&raw)
}

/// Compresses per-processor raw core values into dense ids ordered by each
/// core's lowest processor, which is the order of first appearance.
fn dense_core_ids(raw: &[u32]) -> Option<Box<[CoreId]>> {
    let mut dense_of_raw: Vec<Option<u32>> = vec![None; raw.len()];
    let mut next = 0u32;
    let mut ids = Vec::with_capacity(raw.len());
    for &value in raw {
        // Raw values are record ordinals or processor ids, so they fit
        // `0..len`; a value outside that range came from a malformed table.
        let slot = dense_of_raw.get_mut(usize::try_from(value).ok()?)?;
        if slot.is_none() {
            *slot = Some(next);
            next = next.checked_add(1)?;
        }
        ids.push(CoreId::new((*slot)?));
    }
    (!ids.is_empty()).then(|| ids.into_boxed_slice())
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{cores_from_sibling_lists, MAX_PROCESSOR_ID};
    use crate::topology::types::CoreId;
    use std::fs;

    const CPU_ROOT: &str = "/sys/devices/system/cpu";

    pub(super) fn detect(logical_processors: usize) -> Option<Box<[CoreId]>> {
        if logical_processors == 0 || logical_processors > MAX_PROCESSOR_ID {
            return None;
        }
        let lists: Vec<Option<String>> = (0..logical_processors)
            .map(|processor| {
                fs::read_to_string(format!(
                    "{CPU_ROOT}/cpu{processor}/topology/thread_siblings_list"
                ))
                .ok()
            })
            .collect();
        let lists: Vec<Option<&str>> = lists.iter().map(|list| list.as_deref()).collect();
        cores_from_sibling_lists(&lists, logical_processors)
    }
}

#[cfg(test)]
mod tests {
    use super::super::efficiency::records::fixtures::{
        cache_record, core_record, homogeneous_host_buffer, hybrid_host_buffer,
    };
    use super::{cores_from_processor_records, dense_core_ids};
    use crate::topology::types::CoreId;

    fn ids(table: &[CoreId]) -> Vec<u32> {
        table.iter().map(|id| id.get()).collect()
    }

    #[test]
    fn two_thread_cores_pair_their_processors() {
        let table = cores_from_processor_records(&homogeneous_host_buffer(), 16)
            .expect("eight two-thread cores form a table");
        assert_eq!(
            ids(&table),
            vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7]
        );
    }

    #[test]
    fn single_thread_cores_are_present_not_absent() {
        // No SMT is a reported fact, one processor per core, not a missing
        // report.
        let table = cores_from_processor_records(&hybrid_host_buffer(), 24)
            .expect("24 single-thread cores form a table");
        assert_eq!(ids(&table), (0..24).collect::<Vec<u32>>());
    }

    #[test]
    fn core_ids_follow_lowest_processor_not_record_order() {
        // The platform listed the core holding processors 2,3 before the one
        // holding 0,1. Dense ids still start from the lowest processor, so a
        // consumer iterating processors in order meets core 0 first.
        let records = [
            core_record(0, &[(0, 0b1100)]),
            cache_record(),
            core_record(0, &[(0, 0b0011)]),
        ]
        .concat();
        let table = cores_from_processor_records(&records, 4).expect("two cores");
        assert_eq!(ids(&table), vec![0, 0, 1, 1]);
    }

    #[test]
    fn a_processor_claimed_by_two_records_is_absent() {
        let records = [
            core_record(0, &[(0, 0b0011)]),
            core_record(0, &[(0, 0b0010)]),
        ]
        .concat();
        assert_eq!(cores_from_processor_records(&records, 2), None);
    }

    #[test]
    fn records_that_do_not_cover_every_processor_are_absent() {
        let records = core_record(0, &[(0, 0b0011)]);
        assert_eq!(cores_from_processor_records(&records, 3), None);
        assert_eq!(cores_from_processor_records(&records, 0), None);
        // Processors 0,1 and 64,65 leave a hole in 0..66.
        let gapped = [core_record(0, &[(0, 0b11)]), core_record(0, &[(1, 0b11)])].concat();
        assert_eq!(cores_from_processor_records(&gapped, 66), None);
    }

    #[test]
    fn dense_ids_are_by_first_appearance() {
        // Raw values are record ordinals or lowest-processor ids, both below
        // the table length; the dense id is the order of first appearance.
        let table = dense_core_ids(&[3, 3, 1, 1, 3, 5]).expect("three cores");
        assert_eq!(ids(&table), vec![0, 0, 1, 1, 0, 2]);
        assert_eq!(dense_core_ids(&[]), None);
        // A raw value outside 0..len came from a malformed table.
        assert_eq!(dense_core_ids(&[0, 99]), None);
    }

    #[cfg(any(target_os = "linux", windows))]
    mod sibling_lists {
        use super::super::cores_from_sibling_lists;
        use super::ids;

        #[test]
        fn symmetric_sibling_lists_pair_their_processors() {
            let lists = [Some("0-1"), Some("0,1"), Some("2-3"), Some("2-3")];
            let table = cores_from_sibling_lists(&lists, 4).expect("two cores");
            assert_eq!(ids(&table), vec![0, 0, 1, 1]);
        }

        #[test]
        fn singleton_lists_are_one_processor_per_core() {
            let lists = [Some("0"), Some("1"), Some("2")];
            let table = cores_from_sibling_lists(&lists, 3).expect("three cores");
            assert_eq!(ids(&table), vec![0, 1, 2]);
        }

        #[test]
        fn an_absent_file_or_asymmetric_list_is_absent() {
            assert_eq!(cores_from_sibling_lists(&[Some("0-1"), None], 2), None);
            // 1 claims 0 as a sibling but 0 does not claim 1.
            assert_eq!(cores_from_sibling_lists(&[Some("0"), Some("0-1")], 2), None);
            // A list that omits its own processor.
            assert_eq!(cores_from_sibling_lists(&[Some("1"), Some("1")], 2), None);
        }

        #[test]
        fn members_beyond_the_snapshot_are_absent() {
            assert_eq!(cores_from_sibling_lists(&[Some("0,5")], 1), None);
            assert_eq!(cores_from_sibling_lists(&[Some("0")], 2), None);
        }
    }
}
