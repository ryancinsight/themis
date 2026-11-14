//! Windows backend: `SetThreadGroupAffinity` and `GetThreadGroupAffinity` on
//! the calling thread, with the request narrowed to the group's active
//! processors.

use core::ffi::c_void;
use core::mem::size_of;

use super::BindError;
use crate::topology::cpu::ProcessorGroupAffinity;

/// `GROUP_AFFINITY` from `winnt.h`.
#[repr(C)]
struct GroupAffinity {
    mask: usize,
    group: u16,
    reserved: [u16; 3],
}

// SAFETY: the declarations match the Win32 signatures (`processthreadsapi.h`,
// `processtopologyapi.h`, `sysinfoapi.h`, `errhandlingapi.h`): `GetCurrentThread`
// returns a pseudo `HANDLE`; the two group-affinity calls take that handle and
// `GROUP_AFFINITY` pointers and return a `BOOL`;
// `GetLogicalProcessorInformationEx` takes a `u32` relationship code, an output
// buffer and an in/out `u32` length and returns a `BOOL`; `GetLastError`
// returns a `DWORD`. `GroupAffinity` above is `#[repr(C)]` with the same field
// order and widths as the native structure.
extern "system" {
    fn GetCurrentThread() -> *mut c_void;
    fn SetThreadGroupAffinity(
        thread: *mut c_void,
        group_affinity: *const GroupAffinity,
        previous_group_affinity: *mut GroupAffinity,
    ) -> i32;
    fn GetThreadGroupAffinity(thread: *mut c_void, group_affinity: *mut GroupAffinity) -> i32;
    fn GetLogicalProcessorInformationEx(
        relationship: u32,
        buffer: *mut c_void,
        returned_length: *mut u32,
    ) -> i32;
    fn GetLastError() -> u32;
}

const PROCESSORS_PER_GROUP: u32 = 64;
/// `RelationGroup` of `LOGICAL_PROCESSOR_RELATIONSHIP`.
const RELATION_GROUP: u32 = 4;
/// `ERROR_BAD_LENGTH`: the reply is shorter than its own header claims.
const ERROR_BAD_LENGTH: i32 = 24;

// The `RelationGroup` reply is one record: an 8-byte header (relationship,
// size), the 24-byte `GROUP_RELATIONSHIP` prefix (`u16` maximum and active
// group counts, 20 reserved bytes), then one 48-byte `PROCESSOR_GROUP_INFO` per
// active group, whose `ActiveProcessorMask` sits at byte 40 (two count bytes,
// 38 reserved). A 64-bit `KAFFINITY` is assumed: Themis configures no 32-bit
// Windows target. 512 groups is the crate's 32 768-processor bound.
const HEADER_BYTES: usize = 8;
const ACTIVE_GROUPS_OFFSET: usize = HEADER_BYTES + 2;
const GROUP_INFOS_OFFSET: usize = HEADER_BYTES + 24;
const GROUP_INFO_BYTES: usize = 48;
const ACTIVE_MASK_OFFSET: usize = 40;
const MAX_GROUPS: usize = 512;
const REPLY_BYTES: usize = GROUP_INFOS_OFFSET + MAX_GROUPS * GROUP_INFO_BYTES;
const _: () = assert!(REPLY_BYTES == 24_608 && size_of::<usize>() == 8);

/// The processors one `SetThreadGroupAffinity` call can express: a single
/// processor group and a native mask within it.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(super) struct Mask {
    group: u16,
    mask: usize,
}

impl Mask {
    pub(super) fn from_group(group: ProcessorGroupAffinity) -> Self {
        Self {
            group: group.group(),
            mask: group.mask(),
        }
    }

    pub(super) fn from_processors(
        mut processors: impl Iterator<Item = u32>,
    ) -> Result<Self, BindError> {
        let mut result = Self { group: 0, mask: 0 };
        let mut first = true;
        while let Some(processor) = processors.next() {
            let single = ProcessorGroupAffinity::from_processor(processor)
                .ok_or(BindError::OutOfRange { processor })?;
            if first {
                result.group = single.group();
                first = false;
            } else if single.group() != result.group {
                return Err(BindError::SpansGroups {
                    groups: distinct_groups(result.group, single.group(), processors),
                });
            }
            result.mask |= single.mask();
        }
        Ok(result)
    }

    pub(super) fn contains(&self, processor: u32) -> bool {
        ProcessorGroupAffinity::from_processor(processor)
            .is_some_and(|single| single.group() == self.group && self.mask & single.mask() != 0)
    }

    pub(super) fn len(&self) -> u32 {
        self.mask.count_ones()
    }

    pub(super) fn processors(&self) -> impl Iterator<Item = u32> + '_ {
        (0..usize::BITS)
            .filter(|bit| self.mask >> bit & 1 == 1)
            .map(|bit| u32::from(self.group) * PROCESSORS_PER_GROUP + bit)
    }
}

/// Counts the distinct groups named by two known groups and the unread rest of
/// an input, in a 65 536-bit stack set. Runs only on the refusal path.
#[inline(never)]
fn distinct_groups(first: u16, second: u16, rest: impl Iterator<Item = u32>) -> u32 {
    let mut seen = [0u64; 1024];
    let groups = [first, second]
        .into_iter()
        .chain(rest.filter_map(|processor| u16::try_from(processor / PROCESSORS_PER_GROUP).ok()));
    for group in groups {
        if let Some(word) = seen.get_mut(usize::from(group) / 64) {
            *word |= 1 << (group % 64);
        }
    }
    seen.iter().map(|word| word.count_ones()).sum()
}

/// Reads `GetLastError` right after the failing call, before any other call on
/// the thread can overwrite it.
fn win32_error() -> BindError {
    // SAFETY: `GetLastError` has no preconditions and reads the calling
    // thread's error slot.
    let code = unsafe { GetLastError() };
    BindError::Os {
        code: i32::from_ne_bytes(code.to_ne_bytes()),
    }
}

/// Reports the processors the calling thread may currently run on. A thread
/// never bound on a host with more than 64 processors reports only its primary
/// group.
pub(super) fn current() -> Result<Mask, BindError> {
    let mut affinity = GroupAffinity {
        mask: 0,
        group: 0,
        reserved: [0; 3],
    };
    // SAFETY: the pseudo-handle is valid for the calling thread, and the call
    // writes one `GROUP_AFFINITY` through a valid, exclusive out-pointer.
    let succeeded = unsafe { GetThreadGroupAffinity(GetCurrentThread(), &mut affinity) };
    if succeeded == 0 {
        Err(win32_error())
    } else {
        Ok(Mask {
            group: affinity.group,
            mask: affinity.mask,
        })
    }
}

/// The `ActiveProcessorMask` of `group`, or `NoActiveProcessor` for a group
/// the host does not have.
#[expect(
    clippy::large_stack_arrays,
    reason = "the 24 608-byte reply bound keeps restore allocation-free (ADR 0006)"
)]
fn active_mask(group: u16) -> Result<usize, BindError> {
    let truncated = BindError::Os {
        code: ERROR_BAD_LENGTH,
    };
    let mut reply = [0u8; REPLY_BYTES];
    let mut length = u32::try_from(REPLY_BYTES).map_err(|_| truncated)?;
    // SAFETY: `reply` is a live, initialized buffer of exactly `length` bytes,
    // `length` is a valid in/out pointer, and the call writes only within the
    // advertised length, failing with `ERROR_INSUFFICIENT_BUFFER` otherwise.
    let succeeded = unsafe {
        GetLogicalProcessorInformationEx(RELATION_GROUP, reply.as_mut_ptr().cast(), &mut length)
    };
    if succeeded == 0 {
        return Err(win32_error());
    }
    let active_groups = reply
        .get(ACTIVE_GROUPS_OFFSET..ACTIVE_GROUPS_OFFSET + 2)
        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
        .map(u16::from_ne_bytes)
        .ok_or(truncated)?;
    if group >= active_groups {
        return Err(BindError::NoActiveProcessor);
    }
    let offset = GROUP_INFOS_OFFSET + usize::from(group) * GROUP_INFO_BYTES + ACTIVE_MASK_OFFSET;
    reply
        .get(offset..offset + size_of::<usize>())
        .and_then(|bytes| <[u8; size_of::<usize>()]>::try_from(bytes).ok())
        .map(usize::from_ne_bytes)
        .ok_or(truncated)
}

/// Confines the calling thread to the active processors of `mask`; on `Err`
/// its affinity is unchanged.
pub(super) fn apply(mask: &Mask) -> Result<(), BindError> {
    let narrowed = mask.mask & active_mask(mask.group)?;
    if narrowed == 0 {
        return Err(BindError::NoActiveProcessor);
    }
    let requested = GroupAffinity {
        mask: narrowed,
        group: mask.group,
        reserved: [0; 3],
    };
    // SAFETY: `GetCurrentThread` has no preconditions and returns a
    // pseudo-handle valid for the calling thread. `requested` is a live,
    // initialized `GROUP_AFFINITY` that the call only reads, and the previous
    // affinity out-pointer is documented as optional, so null is valid.
    let succeeded =
        unsafe { SetThreadGroupAffinity(GetCurrentThread(), &requested, core::ptr::null_mut()) };
    if succeeded == 0 {
        Err(win32_error())
    } else {
        Ok(())
    }
}
