//! Platform CPU-locality probes.

use crate::law::NumaNodeId;

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
    #[cfg(all(feature = "std", target_os = "linux"))]
    {
        let mut cpu = 0u32;
        let mut node = 0u32;
        // SAFETY: `getcpu` is a standard glibc/musl library function.
        // It writes two `u32` outputs through valid pointers and does not
        // retain them after the call.
        unsafe {
            extern "C" {
                fn getcpu(cpu: *mut u32, node: *mut u32, tcache: *mut core::ffi::c_void) -> i32;
            }
            if getcpu(&mut cpu, &mut node, core::ptr::null_mut()) == 0 {
                Some(CpuLocality {
                    processor: cpu,
                    numa_node: NumaNodeId::new(node),
                })
            } else {
                None
            }
        }
    }

    #[cfg(all(feature = "std", windows))]
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
            extern "system" {
                fn GetCurrentProcessorNumberEx(proc_number: *mut ProcessorNumber);
                fn GetNumaProcessorNodeEx(processor: *const ProcessorNumber, node_number: *mut u16) -> i32;
            }
            let mut proc_num = ProcessorNumber { group: 0, number: 0, reserved: 0 };
            GetCurrentProcessorNumberEx(&mut proc_num);
            let mut node = 0u16;
            if GetNumaProcessorNodeEx(&proc_num, &mut node) != 0 {
                let system_processor = (proc_num.group as u32) * 64 + (proc_num.number as u32);
                Some(CpuLocality {
                    processor: system_processor,
                    numa_node: NumaNodeId::new(node as u32),
                })
            } else {
                None
            }
        }
    }

    #[cfg(not(any(
        all(feature = "std", target_os = "linux"),
        all(feature = "std", windows)
    )))]
    {
        None
    }
}
