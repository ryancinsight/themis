//! Windows logical-processor cache discovery.

use crate::topology::cpu::MAX_PROCESSOR_ID;
use crate::topology::types::CacheLevel;
use core::mem::size_of;
use std::ffi::c_void;

const RELATION_CACHE: u32 = 2;
const RECORD_HEADER_BYTES: usize = 8;
/// `PROCESSOR_CACHE_TYPE` offset within a `SYSTEM_LOGICAL_PROCESSOR_
/// INFORMATION_EX` cache record: the 8-byte record header, then
/// `CACHE_RELATIONSHIP`'s `Level`, `Associativity`, `LineSize`, and
/// `CacheSize` (1 + 1 + 2 + 4). The two fields either side of it are already
/// read at offsets 10 and 12, and `Reserved[18]` runs from 20 to the group
/// count at 38.
const CACHE_TYPE_OFFSET: usize = 16;
/// `PROCESSOR_CACHE_TYPE` discriminants (`winnt.h`).
const CACHE_UNIFIED: u32 = 0;
const CACHE_DATA: u32 = 2;
const CACHE_PREFIX_BYTES: usize = 32;
const GROUP_AFFINITY_BYTES: usize = 16;
const GROUP_MASK_OFFSET: usize = RECORD_HEADER_BYTES + CACHE_PREFIX_BYTES;
const MAX_GROUPS: usize = 64;
const MAX_BUFFER_BYTES: usize = 1024 * 1024;

// SAFETY: the declarations match the Win32 signatures for these entry points
// (`GetLogicalProcessorInformationEx`, `winnt.h`): `u32` relationship code,
// caller-provided output buffer, in/out length pointer, `BOOL` result. A
// mismatched signature here would corrupt the stack at every call site.
extern "system" {
    fn GetLogicalProcessorInformationEx(
        relationship_type: u32,
        buffer: *mut c_void,
        returned_length: *mut u32,
    ) -> i32;
}

pub(super) fn detect() -> Option<Box<[CacheLevel]>> {
    let mut returned_length = 0u32;
    // SAFETY: The null buffer and valid output length pointer request the
    // required byte count without writing through the buffer pointer.
    let first_call = unsafe {
        GetLogicalProcessorInformationEx(
            RELATION_CACHE,
            core::ptr::null_mut(),
            core::ptr::addr_of_mut!(returned_length),
        )
    };
    if first_call != 0 {
        return None;
    }
    let buffer_length = usize::try_from(returned_length).ok()?;
    if buffer_length == 0 || buffer_length > MAX_BUFFER_BYTES {
        return None;
    }

    let mut buffer = vec![0u8; buffer_length];
    // SAFETY: `buffer` is writable for `buffer_length` bytes, and the API was
    // queried immediately above for this exact relationship and capacity.
    let second_call = unsafe {
        GetLogicalProcessorInformationEx(
            RELATION_CACHE,
            buffer.as_mut_ptr().cast(),
            core::ptr::addr_of_mut!(returned_length),
        )
    };
    if second_call == 0 {
        return None;
    }
    let used = usize::try_from(returned_length).ok()?;
    let bytes = buffer.get(..used.min(buffer.len()))?;
    parse_records(bytes)
}

fn parse_records(bytes: &[u8]) -> Option<Box<[CacheLevel]>> {
    let mut offset = 0usize;
    let mut levels = Vec::with_capacity(8);
    while offset < bytes.len() {
        let relationship = u32::from_ne_bytes(field::<4>(bytes, offset)?);
        let record_size =
            usize::try_from(u32::from_ne_bytes(field::<4>(bytes, offset + 4)?)).ok()?;
        if !(RECORD_HEADER_BYTES..=MAX_BUFFER_BYTES).contains(&record_size)
            || record_size > bytes.len().saturating_sub(offset)
        {
            return None;
        }
        if relationship == RELATION_CACHE {
            let record = bytes.get(offset..offset + record_size)?;
            if let Some(level) = parse_cache_record(record) {
                if !levels
                    .iter()
                    .any(|existing: &CacheLevel| existing == &level)
                {
                    levels.push(level);
                }
            }
        }
        offset = offset.checked_add(record_size)?;
    }
    (!levels.is_empty()).then(|| levels.into_boxed_slice())
}

fn parse_cache_record(record: &[u8]) -> Option<CacheLevel> {
    if record.len() < GROUP_MASK_OFFSET + GROUP_AFFINITY_BYTES {
        return None;
    }
    let level = u32::from(*record.get(RECORD_HEADER_BYTES)?);
    if !(1..=3).contains(&level) {
        return None;
    }
    let size_bytes = usize::try_from(u32::from_ne_bytes(field::<4>(record, 12)?)).ok()?;
    if size_bytes == 0 {
        return None;
    }

    // Report only caches that hold data. Windows reports an instruction cache
    // with the same `Level` as its sibling data cache, and this type is the
    // only field distinguishing them: on a split-L1 machine both arrive as
    // "level 1", differing only in size. A consumer reducing levels by `level`
    // then silently resolves L1 to whichever entry it happened to scan last.
    //
    // That is not hypothetical. On a hybrid Arrow Lake host this reported
    // 48 KiB (P-core L1d), 32 KiB (E-core L1d) and 64 KiB (L1i) all as level 1,
    // and leto's `CacheGeometry::l1_bytes` — a published figure meant for data
    // blocking — resolved to the 64 KiB instruction cache.
    //
    // Instruction and trace caches carry no data-placement meaning, which is
    // what this crate observes, so they are excluded rather than exposed. If a
    // consumer ever needs the distinction, publishing the type on `CacheLevel`
    // is a breaking change to a struct with public fields and belongs in its
    // own record.
    let cache_type = u32::from_ne_bytes(field::<4>(record, CACHE_TYPE_OFFSET)?);
    if cache_type != CACHE_UNIFIED && cache_type != CACHE_DATA {
        return None;
    }
    let line_size = usize::from(u16::from_ne_bytes(field::<2>(record, 10)?));
    let group_count = usize::from(u16::from_ne_bytes(field::<2>(record, 38)?));
    if !(1..=MAX_GROUPS).contains(&group_count) {
        return None;
    }

    let group_bytes = group_count.checked_mul(GROUP_AFFINITY_BYTES)?;
    let group_end = GROUP_MASK_OFFSET.checked_add(group_bytes)?;
    if group_end > record.len() {
        return None;
    }
    let mut shared_processors = Vec::with_capacity(group_count);
    for group_index in 0..group_count {
        let group_offset = GROUP_MASK_OFFSET + group_index * GROUP_AFFINITY_BYTES;
        let mask = affinity_mask(record, group_offset)?;
        let group = u16::from_ne_bytes(field::<2>(record, group_offset + 8)?);
        for bit in 0..usize::BITS as usize {
            if mask & (1usize << bit) == 0 {
                continue;
            }
            let processor = usize::from(group).checked_mul(64)?.checked_add(bit)?;
            if processor < MAX_PROCESSOR_ID {
                shared_processors.push(u32::try_from(processor).ok()?);
            }
        }
    }
    if shared_processors.is_empty() {
        return None;
    }

    Some(CacheLevel {
        level,
        size_bytes,
        line_bytes: (line_size > 0).then_some(line_size),
        shared_processors: shared_processors.into_boxed_slice(),
    })
}

fn affinity_mask(bytes: &[u8], offset: usize) -> Option<usize> {
    match size_of::<usize>() {
        8 => usize::try_from(u64::from_ne_bytes(field::<8>(bytes, offset)?)).ok(),
        4 => usize::try_from(u32::from_ne_bytes(field::<4>(bytes, offset)?)).ok(),
        _ => None,
    }
}

fn field<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    bytes.get(offset..offset.checked_add(N)?)?.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::{parse_cache_record, CACHE_TYPE_OFFSET, GROUP_AFFINITY_BYTES, GROUP_MASK_OFFSET};

    const CACHE_INSTRUCTION: u32 = 1;
    const CACHE_TRACE: u32 = 3;

    /// Build one `SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX` cache record with a
    /// single-group affinity mask, mirroring the field offsets the parser reads.
    fn cache_record(level: u8, size_bytes: u32, line_size: u16, cache_type: u32) -> Vec<u8> {
        let record_size = GROUP_MASK_OFFSET + GROUP_AFFINITY_BYTES;
        let mut record = vec![0u8; record_size];
        record[0..4].copy_from_slice(&2u32.to_ne_bytes()); // RelationCache
        let size_field = u32::try_from(record_size).expect("record size fits in u32");
        record[4..8].copy_from_slice(&size_field.to_ne_bytes());
        record[8] = level;
        record[9] = 8; // associativity
        record[10..12].copy_from_slice(&line_size.to_ne_bytes());
        record[12..16].copy_from_slice(&size_bytes.to_ne_bytes());
        record[CACHE_TYPE_OFFSET..CACHE_TYPE_OFFSET + 4].copy_from_slice(&cache_type.to_ne_bytes());
        record[38..40].copy_from_slice(&1u16.to_ne_bytes()); // group count
        record[GROUP_MASK_OFFSET..GROUP_MASK_OFFSET + 8].copy_from_slice(&0b11u64.to_ne_bytes()); // processors 0 and 1
        record
    }

    #[test]
    fn data_and_unified_caches_are_reported() {
        let data = parse_cache_record(&cache_record(1, 49152, 64, super::CACHE_DATA))
            .expect("data cache is reported");
        assert_eq!(data.level, 1);
        assert_eq!(data.size_bytes, 49152);
        assert_eq!(data.line_bytes, Some(64));
        assert_eq!(&*data.shared_processors, &[0, 1]);

        let unified = parse_cache_record(&cache_record(3, 37_748_736, 64, super::CACHE_UNIFIED))
            .expect("unified cache is reported");
        assert_eq!(unified.level, 3);
        assert_eq!(unified.size_bytes, 37_748_736);
    }

    #[test]
    fn instruction_and_trace_caches_are_excluded() {
        // Windows reports the L1 instruction cache with the same `Level` as the
        // L1 data cache, so without this filter a level-keyed reduction can
        // resolve L1 to an instruction size.
        assert!(parse_cache_record(&cache_record(1, 65536, 64, CACHE_INSTRUCTION)).is_none());
        assert!(parse_cache_record(&cache_record(1, 65536, 64, CACHE_TRACE)).is_none());
    }

    #[test]
    fn a_split_l1_pair_resolves_to_the_data_cache_alone() {
        // The exact hybrid-host shape that produced the defect: a 48 KiB data
        // cache and a 64 KiB instruction cache, both level 1.
        let records = [
            cache_record(1, 49152, 64, super::CACHE_DATA),
            cache_record(1, 65536, 64, CACHE_INSTRUCTION),
        ];
        let reported: Vec<_> = records
            .iter()
            .filter_map(|record| parse_cache_record(record))
            .collect();
        assert_eq!(reported.len(), 1, "only the data cache survives");
        assert_eq!(reported[0].size_bytes, 49152);
    }
}
