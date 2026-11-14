//! Platform CPU-locality probes.

use crate::law::NumaNodeId;
use crate::topology::MAX_NUMA_NODE_IDS;

/// Returns the current processor when the platform exposes it.
///
/// Independent of the NUMA-node lookup: a processor whose node the platform
/// cannot report, or reports beyond the crate's node bound, is still reported.
/// Available without the `std` feature.
#[must_use]
#[inline]
pub fn current_processor() -> Option<u32> {
    query_processor_os()
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
    query_numa_node_os()
}

#[inline]
pub(super) fn query_numa_node_or_default() -> NumaNodeId {
    try_current_numa_node().unwrap_or(NumaNodeId::ZERO)
}

/// Processor ids the crate reports, exclusive: the bound the topology
/// detection paths enforce (`MAX_PROCESSOR_ID`).
#[cfg(all(any(target_os = "linux", windows), not(miri)))]
const PROCESSOR_BOUND: u32 = 32_768;

fn query_processor_os() -> Option<u32> {
    query_os(false).map(|raw| raw.processor)
}

fn query_numa_node_os() -> Option<NumaNodeId> {
    let node = query_os(true)?.node?;
    usize::try_from(node)
        .is_ok_and(|node| node < MAX_NUMA_NODE_IDS)
        .then(|| NumaNodeId::new(node))
}

/// The OS report before the node bound is applied; `node` is `None` unless
/// requested and reported.
struct RawLocality {
    processor: u32,
    node: Option<u32>,
}

// Miri does not emulate the raw `getcpu` syscall or
// `GetCurrentProcessorNumberEx` ("unsupported operation"); under Miri neither
// OS-specific arm below compiles, falling through to the "unsupported platform"
// `None` arm -- consistent with this module's contract (unreported locality is
// `None`, never fabricated), and Miri is not a real platform to report
// locality for.
#[cfg(all(target_os = "linux", not(miri)))]
#[inline(never)]
fn query_os(with_node: bool) -> Option<RawLocality> {
    let mut cpu = 0u32;
    let mut node = 0u32;
    let node_out = if with_node {
        core::ptr::addr_of_mut!(node)
    } else {
        core::ptr::null_mut()
    };
    // The kernel entry point, not a libc `getcpu` wrapper: musl exports
    // none (its `sched_getcpu` reads the vDSO or issues this syscall
    // internally and reports no node), so a library declaring `getcpu`
    // fails to load on every musllinux image ("symbol not found" at
    // import). The syscall exists on every Linux kernel since 2.6.19 and
    // under every libc.
    //
    // SAFETY: `SYS_getcpu` takes `(unsigned *cpu, unsigned *node,
    // struct getcpu_cache *unused)`. `cpu` is a valid, writable, aligned
    // `u32` local; `node_out` is either the same kind of local or null, which
    // the kernel accepts for an unwanted node; the kernel writes one `u32`
    // through each non-null pointer and retains none, and the third argument
    // has been ignored by the kernel since 2.6.24 and is null.
    let status = unsafe {
        libc::syscall(
            libc::SYS_getcpu,
            core::ptr::addr_of_mut!(cpu),
            node_out,
            core::ptr::null_mut::<core::ffi::c_void>(),
        )
    };
    (status == 0 && cpu < PROCESSOR_BOUND).then_some(RawLocality {
        processor: cpu,
        node: with_node.then_some(node),
    })
}

#[cfg(all(windows, not(miri)))]
#[inline(never)]
fn query_os(with_node: bool) -> Option<RawLocality> {
    #[repr(C)]
    struct ProcessorNumber {
        group: u16,
        number: u8,
        reserved: u8,
    }
    // SAFETY: the declarations match the Win32 signatures for these entry
    // points (`GetCurrentProcessorNumberEx`, `GetNumaProcessorNodeEx`,
    // `processthreadsapi.h` / `systemtopologyapi.h`): out-pointer to
    // `PROCESSOR_NUMBER`, `PROCESSOR_NUMBER` in-pointer with a `USHORT`
    // out-pointer, `BOOL` result. `ProcessorNumber` above is `#[repr(C)]`
    // with the same field order and widths.
    extern "system" {
        fn GetCurrentProcessorNumberEx(proc_number: *mut ProcessorNumber);
        fn GetNumaProcessorNodeEx(processor: *const ProcessorNumber, node_number: *mut u16) -> i32;
    }
    let mut proc_num = ProcessorNumber {
        group: 0,
        number: 0,
        reserved: 0,
    };
    // SAFETY: the call writes one `PROCESSOR_NUMBER` through a valid, exclusive
    // out-pointer and retains it never; it has no failure return.
    unsafe { GetCurrentProcessorNumberEx(core::ptr::addr_of_mut!(proc_num)) };
    let processor = u32::from(proc_num.group) * 64 + u32::from(proc_num.number);
    if processor >= PROCESSOR_BOUND {
        return None;
    }
    let node = if with_node {
        let mut node = 0u16;
        // SAFETY: reads the `PROCESSOR_NUMBER` local above and writes one node
        // through a valid, exclusive out-pointer; neither pointer is retained.
        let reported = unsafe {
            GetNumaProcessorNodeEx(core::ptr::addr_of!(proc_num), core::ptr::addr_of_mut!(node))
        };
        (reported != 0).then_some(u32::from(node))
    } else {
        None
    };
    Some(RawLocality { processor, node })
}

#[cfg(not(any(all(target_os = "linux", not(miri)), all(windows, not(miri)))))]
fn query_os(_with_node: bool) -> Option<RawLocality> {
    None
}
