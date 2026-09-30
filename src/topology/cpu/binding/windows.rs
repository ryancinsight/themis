//! Windows backend: `SetThreadGroupAffinity` on the calling thread.

use core::ffi::c_void;

use super::{last_os_error, BindError};
use crate::topology::cpu::ProcessorGroupAffinity;

/// `GROUP_AFFINITY` from `winnt.h`.
#[repr(C)]
struct GroupAffinity {
    mask: usize,
    group: u16,
    reserved: [u16; 3],
}

// SAFETY: the declarations match the Win32 signatures (`processthreadsapi.h`,
// `winbase.h`): `GetCurrentThread` returns a pseudo `HANDLE`, the two group
// affinity calls take that handle and `GROUP_AFFINITY` pointers and return a
// `BOOL`. `GroupAffinity` above is `#[repr(C)]` with the same field order and
// widths as the native structure.
extern "system" {
    fn GetCurrentThread() -> *mut c_void;
    fn SetThreadGroupAffinity(
        thread: *mut c_void,
        group_affinity: *const GroupAffinity,
        previous_group_affinity: *mut GroupAffinity,
    ) -> i32;
}

pub(super) fn bind_current_thread(processor: u32) -> Result<(), BindError> {
    let target = ProcessorGroupAffinity::from_processor(processor)
        .ok_or(BindError::OutOfRange { processor })?;
    let requested = GroupAffinity {
        mask: target.mask(),
        group: target.group(),
        reserved: [0; 3],
    };
    // SAFETY: `GetCurrentThread` has no preconditions and returns a
    // pseudo-handle valid for the calling thread. `requested` is a live,
    // initialized `GROUP_AFFINITY` that the call only reads, and the previous
    // affinity out-pointer is documented as optional, so null is valid.
    let succeeded =
        unsafe { SetThreadGroupAffinity(GetCurrentThread(), &requested, core::ptr::null_mut()) };
    if succeeded == 0 {
        Err(last_os_error())
    } else {
        Ok(())
    }
}

/// Reports the processors the calling thread may currently run on, in Themis
/// numbering, ascending.
#[cfg(test)]
pub(super) fn current_confinement() -> Vec<u32> {
    extern "system" {
        fn GetThreadGroupAffinity(thread: *mut c_void, group_affinity: *mut GroupAffinity) -> i32;
    }

    let mut affinity = GroupAffinity {
        mask: 0,
        group: 0,
        reserved: [0; 3],
    };
    // SAFETY: the pseudo-handle is valid for the calling thread, and the call
    // writes one `GROUP_AFFINITY` through a valid, exclusive out-pointer.
    let succeeded = unsafe { GetThreadGroupAffinity(GetCurrentThread(), &mut affinity) };
    assert_ne!(
        succeeded,
        0,
        "GetThreadGroupAffinity failed: {:?}",
        last_os_error()
    );
    (0..usize::BITS)
        .filter(|bit| affinity.mask >> bit & 1 == 1)
        .map(|bit| u32::from(affinity.group) * 64 + bit)
        .collect()
}
