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
    #[cfg(all(feature = "std", target_os = "linux", target_arch = "x86_64"))]
    {
        let mut cpu = 0u32;
        let mut node = 0u32;
        let ret: isize;
        // SAFETY: `getcpu` writes two `u32` outputs through valid pointers and
        // does not retain them after the syscall returns.
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") 309isize,
                in("rdi") &mut cpu as *mut u32,
                in("rsi") &mut node as *mut u32,
                in("rdx") core::ptr::null_mut::<u8>(),
                lateout("rax") ret,
                lateout("rcx") _,
                lateout("r11") _,
                options(nostack, preserves_flags)
            );
        }
        if ret == 0 {
            Some(CpuLocality {
                processor: cpu,
                numa_node: NumaNodeId::new(node),
            })
        } else {
            None
        }
    }

    #[cfg(all(feature = "std", windows))]
    {
        // SAFETY: `GetCurrentProcessorNumber` takes no pointers and returns the
        // current processor number. `GetNumaProcessorNode` writes one node
        // output during the call and does not retain the pointer.
        unsafe {
            extern "system" {
                fn GetCurrentProcessorNumber() -> u32;
                fn GetNumaProcessorNode(processor: u8, node_number: *mut u8) -> i32;
            }
            let processor = GetCurrentProcessorNumber();
            let mut node = 0u8;
            if GetNumaProcessorNode(processor as u8, &mut node) != 0 {
                Some(CpuLocality {
                    processor,
                    numa_node: NumaNodeId::new(node as u32),
                })
            } else {
                None
            }
        }
    }

    #[cfg(not(any(
        all(feature = "std", target_os = "linux", target_arch = "x86_64"),
        all(feature = "std", windows)
    )))]
    {
        None
    }
}
