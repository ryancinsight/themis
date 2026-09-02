//! Windows core efficiency-class discovery.
//!
//! Reads `GetLogicalProcessorInformationEx(RelationProcessorCore)`, whose
//! `PROCESSOR_RELATIONSHIP` records carry the `EfficiencyClass` byte alongside
//! the `GROUP_AFFINITY` of each core. Walking and assembling those bytes is
//! [`super::records`] and [`super::classes_from_processor_records`]; this file
//! is only the call that produces them.

use super::classes_from_processor_records;
use super::records::RELATION_PROCESSOR_CORE;
use crate::topology::types::EfficiencyClass;
use core::mem::size_of;
use std::ffi::c_void;

const MAX_BUFFER_BYTES: usize = 1024 * 1024;

// SAFETY: the declaration matches the Win32 signature for this entry point
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

pub(super) fn detect(logical_processors: usize) -> Option<Box<[EfficiencyClass]>> {
    classes_from_processor_records(&core_records()?, logical_processors)
}

/// The raw `RelationProcessorCore` buffer, read once per caller.
///
/// Shared with the SMT axis, which walks the same records for their group
/// masks rather than their class byte.
pub(in crate::topology::cpu) fn core_records() -> Option<Vec<u8>> {
    // `super::records` walks the 64-bit `KAFFINITY` layout, where a
    // `GROUP_AFFINITY` is 16 bytes. A 32-bit Windows build packs it into 8 and
    // would be misread field for field, so report absence instead.
    if size_of::<usize>() != 8 {
        return None;
    }

    let mut returned_length = 0u32;
    // SAFETY: The null buffer and valid output length pointer request the
    // required byte count without writing through the buffer pointer.
    let first_call = unsafe {
        GetLogicalProcessorInformationEx(
            RELATION_PROCESSOR_CORE,
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
            RELATION_PROCESSOR_CORE,
            buffer.as_mut_ptr().cast(),
            core::ptr::addr_of_mut!(returned_length),
        )
    };
    if second_call == 0 {
        return None;
    }
    let used = usize::try_from(returned_length).ok()?;
    buffer.truncate(used.min(buffer.len()));
    Some(buffer)
}
