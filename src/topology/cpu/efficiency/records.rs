//! `RelationProcessorCore` record walking for `GetLogicalProcessorInformationEx`.
//!
//! The walk is a pure function over the returned byte buffer, compiled on every
//! target so its fixtures run in every CI leg. Only the call that produces the
//! buffer is Windows-gated.
//!
//! Layout, from `winnt.h`, of one `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX`
//! record whose `Relationship` is `RelationProcessorCore`, at offsets from the
//! start of the record:
//!
//! ```text
//!  0  DWORD Relationship            (0 = RelationProcessorCore)
//!  4  DWORD Size                    (total record bytes, including this header)
//!  8  BYTE  Processor.Flags
//!  9  BYTE  Processor.EfficiencyClass
//! 10  BYTE  Processor.Reserved[20]
//! 30  WORD  Processor.GroupCount
//! 32  GROUP_AFFINITY Processor.GroupMask[GroupCount]
//! ```
//!
//! and of one `GROUP_AFFINITY` at offsets from its own start:
//!
//! ```text
//!  0  KAFFINITY Mask                (pointer-width; 8 bytes on 64-bit Windows)
//!  8  WORD      Group
//! 10  WORD      Reserved[3]
//! ```
//!
//! This walker reads the 64-bit `KAFFINITY` layout. The Windows entry point
//! reports absence rather than misreading a 32-bit buffer.

use crate::topology::cpu::MAX_PROCESSOR_ID;

/// `LOGICAL_PROCESSOR_RELATIONSHIP::RelationProcessorCore`.
pub(super) const RELATION_PROCESSOR_CORE: u32 = 0;

const RECORD_HEADER_BYTES: usize = 8;
const EFFICIENCY_CLASS_OFFSET: usize = 9;
const GROUP_COUNT_OFFSET: usize = 30;
const GROUP_MASK_OFFSET: usize = 32;
/// `GROUP_AFFINITY` size under a 64-bit `KAFFINITY`.
pub(super) const GROUP_AFFINITY_BYTES: usize = 16;
/// Logical processors addressed by one `KAFFINITY` mask, hence one group.
const PROCESSORS_PER_GROUP: usize = 64;
/// A core belongs to exactly one processor group, so `GroupCount` is 1 for a
/// `RelationProcessorCore` record. Accept a small excess rather than rejecting
/// a future record shape outright, but never an unbounded one.
const MAX_GROUPS_PER_CORE: usize = 64;
const MAX_BUFFER_BYTES: usize = 1024 * 1024;

/// One logical processor, the raw `EfficiencyClass` byte of its core, and
/// which core record it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::topology::cpu) struct ProcessorClass {
    /// Processor id in this crate's numbering: `group * 64 + bit`.
    pub(in crate::topology::cpu) processor: u32,
    /// The platform's raw class byte, higher meaning more performant.
    pub(in crate::topology::cpu) raw_class: u8,
    /// Ordinal of the `RelationProcessorCore` record in walk order. Every
    /// processor in one record is an SMT sibling of the others; the SMT axis
    /// reads this, the efficiency axis ignores it.
    pub(in crate::topology::cpu) core: u32,
}

/// Walks a `RelationProcessorCore` buffer into per-processor class bytes.
///
/// Returns `None` on a malformed buffer — a record shorter than its header, a
/// record running past the buffer, a zero or oversized `GroupCount`, or a
/// truncated affinity array. A malformed record is not skipped past: a partial
/// class table would be a fabricated split over the processors it did cover.
pub(in crate::topology::cpu) fn parse_processor_cores(bytes: &[u8]) -> Option<Vec<ProcessorClass>> {
    let mut offset = 0usize;
    let mut entries = Vec::with_capacity(bytes.len() / 64);
    let mut core = 0u32;

    while offset < bytes.len() {
        let relationship = u32::from_ne_bytes(field::<4>(bytes, offset)?);
        let record_size =
            usize::try_from(u32::from_ne_bytes(field::<4>(bytes, offset + 4)?)).ok()?;
        if !(RECORD_HEADER_BYTES..=MAX_BUFFER_BYTES).contains(&record_size)
            || record_size > bytes.len().saturating_sub(offset)
        {
            return None;
        }
        if relationship == RELATION_PROCESSOR_CORE {
            let record = bytes.get(offset..offset.checked_add(record_size)?)?;
            parse_core_record(record, core, &mut entries)?;
            core = core.checked_add(1)?;
        }
        offset = offset.checked_add(record_size)?;
    }

    (!entries.is_empty()).then_some(entries)
}

fn parse_core_record(record: &[u8], core: u32, entries: &mut Vec<ProcessorClass>) -> Option<()> {
    let raw_class = *record.get(EFFICIENCY_CLASS_OFFSET)?;
    let group_count = usize::from(u16::from_ne_bytes(field::<2>(record, GROUP_COUNT_OFFSET)?));
    if !(1..=MAX_GROUPS_PER_CORE).contains(&group_count) {
        return None;
    }

    let group_bytes = group_count.checked_mul(GROUP_AFFINITY_BYTES)?;
    if GROUP_MASK_OFFSET.checked_add(group_bytes)? > record.len() {
        return None;
    }

    for group_index in 0..group_count {
        let group_offset = GROUP_MASK_OFFSET + group_index * GROUP_AFFINITY_BYTES;
        let mask = u64::from_ne_bytes(field::<8>(record, group_offset)?);
        let group = usize::from(u16::from_ne_bytes(field::<2>(record, group_offset + 8)?));
        for bit in 0..PROCESSORS_PER_GROUP {
            if mask & (1u64 << bit) == 0 {
                continue;
            }
            let processor = group.checked_mul(PROCESSORS_PER_GROUP)?.checked_add(bit)?;
            if processor >= MAX_PROCESSOR_ID {
                return None;
            }
            entries.push(ProcessorClass {
                processor: u32::try_from(processor).ok()?,
                raw_class,
                core,
            });
        }
    }

    Some(())
}

fn field<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    bytes.get(offset..offset.checked_add(N)?)?.try_into().ok()
}

/// Recorded `GetLogicalProcessorInformationEx` buffers used by the parser and
/// topology tests. Kept out of the test module itself so sibling modules can
/// drive the whole path from platform bytes to public accessor.
#[cfg(test)]
pub(in crate::topology::cpu) mod fixtures {
    use super::{GROUP_AFFINITY_BYTES, RELATION_PROCESSOR_CORE};

    /// `LOGICAL_PROCESSOR_RELATIONSHIP::RelationCache`, an unrelated record the
    /// same call family returns. Present in fixtures to prove the walker steps
    /// over what it does not own.
    const RELATION_CACHE: u32 = 2;

    /// The developer host from the incident this capability exists to prevent:
    /// 24 logical processors whose performance-core mask is `0xc03c03`, i.e.
    /// `{0, 1, 10, 11, 12, 13, 22, 23}` — deliberately not a contiguous low
    /// range. A consumer that assumed "the low ids are the performance cores"
    /// pinned to cpu 2, which is in fact one of the slowest processors on the
    /// part.
    pub(in crate::topology::cpu) const PERFORMANCE_MASK: u64 = 0xc0_3c03;

    /// Builds one `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` core record.
    pub(in crate::topology::cpu) fn core_record(raw_class: u8, groups: &[(u16, u64)]) -> Vec<u8> {
        let size = 32 + groups.len() * GROUP_AFFINITY_BYTES;
        let mut record = vec![0u8; size];
        record[0..4].copy_from_slice(&RELATION_PROCESSOR_CORE.to_ne_bytes());
        record[4..8].copy_from_slice(
            &u32::try_from(size)
                .expect("fixture size fits")
                .to_ne_bytes(),
        );
        record[8] = 1; // Flags: LTP_PC_SMT, ignored by the walker.
        record[9] = raw_class;
        record[30..32].copy_from_slice(
            &u16::try_from(groups.len())
                .expect("fixture fits")
                .to_ne_bytes(),
        );
        for (index, (group, mask)) in groups.iter().enumerate() {
            let base = 32 + index * GROUP_AFFINITY_BYTES;
            record[base..base + 8].copy_from_slice(&mask.to_ne_bytes());
            record[base + 8..base + 10].copy_from_slice(&group.to_ne_bytes());
        }
        record
    }

    /// A foreign record the walker must step over without interpreting.
    pub(in crate::topology::cpu) fn cache_record() -> Vec<u8> {
        let mut record = vec![0u8; 48];
        record[0..4].copy_from_slice(&RELATION_CACHE.to_ne_bytes());
        record[4..8].copy_from_slice(&48u32.to_ne_bytes());
        record[8] = 1; // Level.
        record
    }

    /// The 24-processor hybrid host, one single-threaded core per processor.
    pub(in crate::topology::cpu) fn hybrid_host_buffer() -> Vec<u8> {
        (0..24u32)
            .map(|processor| {
                let is_performance = PERFORMANCE_MASK & (1u64 << processor) != 0;
                core_record(u8::from(is_performance), &[(0, 1u64 << processor)])
            })
            .collect::<Vec<_>>()
            .concat()
    }

    /// Eight SMT cores of one class: 16 processors, no hybrid split.
    pub(in crate::topology::cpu) fn homogeneous_host_buffer() -> Vec<u8> {
        (0..8u32)
            .map(|core| core_record(0, &[(0, 0b11 << (core * 2))]))
            .collect::<Vec<_>>()
            .concat()
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{
        cache_record, core_record, homogeneous_host_buffer, hybrid_host_buffer, PERFORMANCE_MASK,
    };
    use super::{parse_processor_cores, ProcessorClass};

    fn parsed(bytes: &[u8]) -> Vec<ProcessorClass> {
        parse_processor_cores(bytes).expect("fixture buffer is well formed")
    }

    #[test]
    fn hybrid_host_classes_follow_the_recorded_performance_mask() {
        let entries = parsed(&hybrid_host_buffer());
        assert_eq!(entries.len(), 24);
        for entry in &entries {
            let expected = u8::from(PERFORMANCE_MASK & (1u64 << entry.processor) != 0);
            assert_eq!(
                entry.raw_class, expected,
                "processor {} class",
                entry.processor
            );
        }

        let performance: Vec<u32> = entries
            .iter()
            .filter(|entry| entry.raw_class == 1)
            .map(|entry| entry.processor)
            .collect();
        assert_eq!(performance, vec![0, 1, 10, 11, 12, 13, 22, 23]);
        assert!(
            !performance.contains(&2),
            "processor 2 is an efficiency core on this host"
        );
    }

    #[test]
    fn homogeneous_host_reports_one_class_for_every_processor() {
        let entries = parsed(&homogeneous_host_buffer());
        assert_eq!(entries.len(), 16);
        assert!(entries.iter().all(|entry| entry.raw_class == 0));
        let processors: Vec<u32> = entries.iter().map(|entry| entry.processor).collect();
        assert_eq!(processors, (0..16).collect::<Vec<u32>>());
    }

    #[test]
    fn three_tier_host_preserves_every_distinct_class_byte() {
        let records = [
            core_record(0, &[(0, 0b0001)]), // low-power efficient
            core_record(1, &[(0, 0b0010)]), // efficient
            core_record(2, &[(0, 0b1100)]), // performance, two SMT threads
        ]
        .concat();
        let entries = parsed(&records);
        assert_eq!(
            entries.iter().map(|e| e.raw_class).collect::<Vec<u8>>(),
            vec![0, 1, 2, 2]
        );
    }

    #[test]
    fn multi_group_host_numbers_processors_by_group_and_bit() {
        let records = [
            core_record(1, &[(0, 1u64 << 63)]),
            core_record(0, &[(1, 0b101)]),
            core_record(0, &[(2, 1u64 << 7)]),
        ]
        .concat();
        let entries = parsed(&records);
        assert_eq!(
            entries.iter().map(|e| e.processor).collect::<Vec<u32>>(),
            vec![63, 64, 66, 135]
        );
    }

    #[test]
    fn processors_of_one_record_share_a_core_ordinal_in_walk_order() {
        let records = [
            core_record(0, &[(0, 0b0011)]), // core 0: processors 0, 1
            cache_record(),                 // not a core, does not consume an ordinal
            core_record(0, &[(0, 0b1100)]), // core 1: processors 2, 3
            core_record(1, &[(1, 0b0001)]), // core 2: processor 64
        ]
        .concat();
        let entries = parsed(&records);
        assert_eq!(
            entries
                .iter()
                .map(|e| (e.processor, e.core))
                .collect::<Vec<(u32, u32)>>(),
            vec![(0, 0), (1, 0), (2, 1), (3, 1), (64, 2)]
        );
    }

    #[test]
    fn foreign_records_are_stepped_over_not_interpreted() {
        let records = [
            cache_record(),
            core_record(1, &[(0, 0b01)]),
            cache_record(),
            core_record(0, &[(0, 0b10)]),
        ]
        .concat();
        let entries = parsed(&records);
        assert_eq!(
            entries
                .iter()
                .map(|e| (e.processor, e.raw_class))
                .collect::<Vec<(u32, u8)>>(),
            vec![(0, 1), (1, 0)]
        );
    }

    #[test]
    fn empty_and_relationless_buffers_are_absent() {
        assert_eq!(parse_processor_cores(&[]), None);
        assert_eq!(parse_processor_cores(&cache_record()), None);
    }

    #[test]
    fn malformed_buffers_are_absent_rather_than_partially_parsed() {
        // A record claiming more bytes than the buffer holds.
        let mut oversized = core_record(1, &[(0, 0b1)]);
        oversized[4..8].copy_from_slice(&4096u32.to_ne_bytes());
        assert_eq!(parse_processor_cores(&oversized), None);

        // A record claiming zero size would not advance the walk.
        let mut zero_size = core_record(1, &[(0, 0b1)]);
        zero_size[4..8].copy_from_slice(&0u32.to_ne_bytes());
        assert_eq!(parse_processor_cores(&zero_size), None);

        // A record whose GroupCount outruns the affinity bytes it carries.
        let mut lying_group_count = core_record(1, &[(0, 0b1)]);
        lying_group_count[30..32].copy_from_slice(&4u16.to_ne_bytes());
        assert_eq!(parse_processor_cores(&lying_group_count), None);

        // A record with no affinity array at all.
        let mut no_groups = core_record(1, &[(0, 0b1)]);
        no_groups[30..32].copy_from_slice(&0u16.to_ne_bytes());
        assert_eq!(parse_processor_cores(&no_groups), None);

        // A buffer truncated mid-record: one good record then a stub too short
        // to carry a header.
        let mut truncated = core_record(1, &[(0, 0b1)]);
        truncated.extend_from_slice(&[0u8; 5]);
        assert_eq!(parse_processor_cores(&truncated), None);

        // A well-formed prefix does not license a partial table: the malformed
        // second record fails the whole parse.
        let mut mixed = core_record(1, &[(0, 0b1)]);
        let mut bad = core_record(0, &[(0, 0b10)]);
        bad[30..32].copy_from_slice(&9u16.to_ne_bytes());
        mixed.extend_from_slice(&bad);
        assert_eq!(parse_processor_cores(&mixed), None);
    }

    #[test]
    fn processor_ids_beyond_the_crate_bound_are_absent() {
        // Group 512 puts the first bit at processor 32768, the crate's cap.
        let record = core_record(0, &[(512, 0b1)]);
        assert_eq!(parse_processor_cores(&record), None);
    }
}
