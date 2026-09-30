//! Platform CPU-locality probes.

use crate::law::NumaNodeId;
// Only the two OS-specific arms below consume this, and both are compiled out
// under Miri and on targets with neither backend.
#[cfg(all(feature = "std", any(target_os = "linux", windows), not(miri)))]
use crate::topology::MAX_NUMA_NODE_IDS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CpuLocality {
    processor: u32,
    numa_node: NumaNodeId,
}

/// Returns the current processor when the platform exposes it.
#[must_use]
#[inline]
pub fn current_processor() -> Option<u32> {
    query_cpu_locality_os().map(|locality| locality.processor)
}

/// Queries the calling thread's NUMA node without caching, returning `None`
/// when the platform does not expose the information.
///
/// Unlike [`crate::current_numa_node`], which falls back to node 0 for callers
/// that need a placement decision regardless, this preserves the stack-wide
/// "unreported = `None`, never fabricated" contract for consumers that must
/// distinguish "node 0" from "unknown" (e.g. locality verification).
#[must_use]
#[inline]
pub fn try_current_numa_node() -> Option<NumaNodeId> {
    query_cpu_locality_os().map(|locality| locality.numa_node)
}

#[inline]
pub(super) fn query_numa_node_or_default() -> NumaNodeId {
    try_current_numa_node().unwrap_or(NumaNodeId::ZERO)
}

#[inline(never)]
fn query_cpu_locality_os() -> Option<CpuLocality> {
    // Miri does not emulate the raw `getcpu` syscall or
    // `GetCurrentProcessorNumberEx` ("unsupported operation"); under Miri
    // neither OS-specific arm below compiles, falling through to the existing
    // "unsupported platform" `None` arm -- consistent with this function's own
    // contract (unreported locality is `None`, never fabricated), and Miri is
    // not a real platform to report locality for.
    #[cfg(all(feature = "std", target_os = "linux", not(miri)))]
    {
        let mut cpu = 0u32;
        let mut node = 0u32;
        // The kernel entry point, not the libc `getcpu` wrapper: musl exports
        // the wrapper only from 1.2.5, so a library linking it fails to load
        // on musllinux_1_2 images with a musl older than 1.2.5 ("symbol not
        // found" at import). The syscall exists on every Linux kernel since
        // 2.6.19 and under every libc.
        //
        // SAFETY: `SYS_getcpu` takes `(unsigned *cpu, unsigned *node,
        // struct getcpu_cache *unused)`. The two out-pointers are valid,
        // writable, aligned `u32` locals, the kernel writes one `u32` through
        // each and retains neither, and the third argument has been ignored
        // by the kernel since 2.6.24 and is null.
        let status = unsafe {
            libc::syscall(
                libc::SYS_getcpu,
                core::ptr::addr_of_mut!(cpu),
                core::ptr::addr_of_mut!(node),
                core::ptr::null_mut::<core::ffi::c_void>(),
            )
        };
        if status == 0 && cpu < 32768 && (node as usize) < MAX_NUMA_NODE_IDS {
            Some(CpuLocality {
                processor: cpu,
                numa_node: NumaNodeId::new(node),
            })
        } else {
            None
        }
    }

    #[cfg(all(feature = "std", windows, not(miri)))]
    {
        // SAFETY: `GetCurrentProcessorNumberEx` writes to a valid local struct.
        // `GetNumaProcessorNodeEx` reads from that struct and writes one node
        // output. Neither API retains the pointers after the call.
        unsafe {
            #[repr(C)]
            #[derive(Clone, Copy)]
            struct ProcessorNumber {
                group: u16,
                number: u8,
                reserved: u8,
            }
            // SAFETY: the declarations match the Win32 signatures for these
            // entry points (`GetCurrentProcessorNumberEx`,
            // `GetNumaProcessorNodeEx`, `processthreadsapi.h` /
            // `systemtopologyapi.h`): out-pointer to `PROCESSOR_NUMBER`,
            // `PROCESSOR_NUMBER` in-pointer with a `USHORT` out-pointer, `BOOL`
            // result. `ProcessorNumber` above is `#[repr(C)]` with the same
            // field order and widths.
            extern "system" {
                fn GetCurrentProcessorNumberEx(proc_number: *mut ProcessorNumber);
                fn GetNumaProcessorNodeEx(
                    processor: *const ProcessorNumber,
                    node_number: *mut u16,
                ) -> i32;
            }
            let mut proc_num = ProcessorNumber {
                group: 0,
                number: 0,
                reserved: 0,
            };
            GetCurrentProcessorNumberEx(core::ptr::addr_of_mut!(proc_num));
            let mut node = 0u16;
            if GetNumaProcessorNodeEx(core::ptr::addr_of!(proc_num), core::ptr::addr_of_mut!(node))
                != 0
            {
                let system_processor = u32::from(proc_num.group) * 64 + u32::from(proc_num.number);
                if system_processor < 32768 && (node as usize) < MAX_NUMA_NODE_IDS {
                    Some(CpuLocality {
                        processor: system_processor,
                        numa_node: NumaNodeId::new(u32::from(node)),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    #[cfg(not(any(
        all(feature = "std", target_os = "linux", not(miri)),
        all(feature = "std", windows, not(miri))
    )))]
    {
        None
    }
}
