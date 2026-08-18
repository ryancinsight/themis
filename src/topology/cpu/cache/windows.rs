//! Windows logical-processor cache discovery.

use crate::topology::types::CacheLevel;
use core::mem::size_of;
use std::ffi::c_void;

const RELATION_CACHE: u32 = 2;
const RECORD_HEADER_BYTES: usize = 8;
const CACHE_PREFIX_BYTES: usize = 32;
const GROUP_AFFINITY_BYTES: usize = 16;
const GROUP_MASK_OFFSET: usize = RECORD_HEADER_BYTES + CACHE_PREFIX_BYTES;
const MAX_GROUPS: usize = 64;
const MAX_BUFFER_BYTES: usize = 1024 * 1024;
const MAX_PROCESSOR_ID: usize = 32_768;

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
