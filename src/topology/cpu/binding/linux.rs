//! Linux backend: `sched_setaffinity` on the calling thread.

use core::ffi::c_int;
use core::mem::size_of;

use super::{last_os_error, BindError};
use crate::topology::cpu::MAX_PROCESSOR_ID;

// SAFETY: the declaration matches glibc and musl `sched.h`: `pid_t` is `int`,
// the size is a `size_t` byte count, and the mask is a `cpu_set_t`, which is a
// bitmap of `unsigned long` words. The kernel reads exactly `cpusetsize` bytes
// through `mask` and retains nothing.
extern "C" {
    fn sched_setaffinity(pid: c_int, cpusetsize: usize, mask: *const usize) -> c_int;
}

const WORD_BITS: usize = usize::BITS as usize;

pub(super) fn bind_current_thread(processor: u32) -> Result<(), BindError> {
    let bit = processor as usize;
    if bit >= MAX_PROCESSOR_ID {
        return Err(BindError::OutOfRange { processor });
    }

    // The kernel bitmap is `unsigned long` words, so `usize` words give the
    // right bit order on every endianness. Sizing the mask to the requested
    // bit, not to a fixed 1024-bit `cpu_set_t`, lets hosts past 1024 cpus bind.
    let mut mask = vec![0usize; bit / WORD_BITS + 1];
    mask[bit / WORD_BITS] = 1 << (bit % WORD_BITS);

    // SAFETY: pid 0 names the calling thread. `mask` is a live, initialized
    // allocation of exactly `mask.len() * size_of::<usize>()` bytes, which is
    // the `cpusetsize` passed, and it outlives the call.
    let status = unsafe { sched_setaffinity(0, mask.len() * size_of::<usize>(), mask.as_ptr()) };
    if status == 0 {
        Ok(())
    } else {
        Err(last_os_error())
    }
}

/// Reports the processors the calling thread may currently run on, ascending.
#[cfg(test)]
pub(super) fn current_confinement() -> Vec<u32> {
    extern "C" {
        fn sched_getaffinity(pid: c_int, cpusetsize: usize, mask: *mut usize) -> c_int;
    }

    let mut mask = vec![0usize; MAX_PROCESSOR_ID / WORD_BITS];
    // SAFETY: pid 0 names the calling thread. `mask` is a live, zero-initialized
    // allocation of exactly `mask.len() * size_of::<usize>()` bytes, the size
    // passed, and the kernel writes only within it.
    let status =
        unsafe { sched_getaffinity(0, mask.len() * size_of::<usize>(), mask.as_mut_ptr()) };
    assert_eq!(status, 0, "sched_getaffinity failed: {:?}", last_os_error());
    (0..MAX_PROCESSOR_ID)
        .filter(|bit| mask[bit / WORD_BITS] >> (bit % WORD_BITS) & 1 == 1)
        .map(|bit| u32::try_from(bit).expect("invariant: the bound fits u32"))
        .collect()
}
