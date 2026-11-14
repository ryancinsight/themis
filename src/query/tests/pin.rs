//! Operating-system thread pinning for query tests.
//!
//! These helpers call the OS directly so a processor a test pins to is known
//! independently of the code under test, with or without the `std` feature
//! (the crate's own binding is `std`-gated).

use std::vec::Vec;

/// Processors the calling thread may run on, ascending, in Themis numbering
/// (`group * 64 + bit` on Windows, the cpu index on Linux).
pub(super) fn allowed_processors() -> Vec<u32> {
    os::allowed_processors()
}

/// Confines the calling thread to exactly `processor`.
pub(super) fn pin_to(processor: u32) {
    os::pin_to(processor);
}

#[cfg(target_os = "linux")]
mod os {
    use std::vec::Vec;

    /// Width of glibc's and musl's `cpu_set_t`, in processors.
    const SET_BITS: usize = 1024;

    fn empty_set() -> libc::cpu_set_t {
        // SAFETY: `cpu_set_t` is a plain array of integers; all-zero bits is
        // the empty set that `CPU_ZERO` produces.
        unsafe { core::mem::zeroed() }
    }

    pub(super) fn allowed_processors() -> Vec<u32> {
        let mut set = empty_set();
        // SAFETY: pid 0 names the calling thread, `set` is a live, writable
        // `cpu_set_t`, and the size passed is its size.
        let status = unsafe { libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &mut set) };
        assert_eq!(status, 0, "sched_getaffinity failed");
        (0..SET_BITS)
            .filter(|&cpu| {
                // SAFETY: `cpu` is below `SET_BITS`, the bit width of `set`.
                unsafe { libc::CPU_ISSET(cpu, &set) }
            })
            .map(|cpu| u32::try_from(cpu).expect("invariant: the set width fits u32"))
            .collect()
    }

    pub(super) fn pin_to(processor: u32) {
        let cpu = usize::try_from(processor).expect("invariant: u32 fits usize");
        assert!(
            cpu < SET_BITS,
            "processor {processor} is past the set width"
        );
        let mut set = empty_set();
        // SAFETY: `cpu` is below `SET_BITS`, checked above.
        unsafe { libc::CPU_SET(cpu, &mut set) };
        // SAFETY: pid 0 names the calling thread, `set` is a live, initialized
        // `cpu_set_t` the call only reads, and the size passed is its size.
        let status = unsafe { libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &set) };
        assert_eq!(status, 0, "sched_setaffinity({processor}) failed");
    }
}

#[cfg(windows)]
mod os {
    use core::ffi::c_void;
    use std::vec::Vec;

    /// `GROUP_AFFINITY` from `winnt.h`.
    #[repr(C)]
    struct GroupAffinity {
        mask: usize,
        group: u16,
        reserved: [u16; 3],
    }

    // SAFETY: the declarations match the Win32 signatures (`processthreadsapi.h`,
    // `processtopologyapi.h`): `GetCurrentThread` returns a pseudo `HANDLE`, the
    // two group affinity calls take that handle and `GROUP_AFFINITY` pointers
    // and return a `BOOL`. `GroupAffinity` is `#[repr(C)]` with the native
    // field order and widths.
    extern "system" {
        fn GetCurrentThread() -> *mut c_void;
        fn GetThreadGroupAffinity(thread: *mut c_void, group_affinity: *mut GroupAffinity) -> i32;
        fn SetThreadGroupAffinity(
            thread: *mut c_void,
            group_affinity: *const GroupAffinity,
            previous_group_affinity: *mut GroupAffinity,
        ) -> i32;
    }

    /// Processors in one processor group, the `KAFFINITY` width on 64-bit
    /// Windows.
    const GROUP_WIDTH: u32 = 64;

    pub(super) fn allowed_processors() -> Vec<u32> {
        let mut affinity = GroupAffinity {
            mask: 0,
            group: 0,
            reserved: [0; 3],
        };
        // SAFETY: the pseudo-handle is valid for the calling thread, and the
        // call writes one `GROUP_AFFINITY` through a valid, exclusive
        // out-pointer.
        let succeeded = unsafe { GetThreadGroupAffinity(GetCurrentThread(), &mut affinity) };
        assert_ne!(succeeded, 0, "GetThreadGroupAffinity failed");
        (0..GROUP_WIDTH)
            .filter(|bit| (affinity.mask >> bit) & 1 == 1)
            .map(|bit| u32::from(affinity.group) * GROUP_WIDTH + bit)
            .collect()
    }

    pub(super) fn pin_to(processor: u32) {
        let requested = GroupAffinity {
            mask: 1 << (processor % GROUP_WIDTH),
            group: u16::try_from(processor / GROUP_WIDTH)
                .expect("invariant: a reported processor is in a group below 65_536"),
            reserved: [0; 3],
        };
        // SAFETY: the pseudo-handle is valid for the calling thread,
        // `requested` is a live, initialized `GROUP_AFFINITY` the call only
        // reads, and the previous-affinity out-pointer is optional, so null is
        // valid.
        let succeeded = unsafe {
            SetThreadGroupAffinity(GetCurrentThread(), &requested, core::ptr::null_mut())
        };
        assert_ne!(succeeded, 0, "SetThreadGroupAffinity({processor}) failed");
    }
}
