# ADR 0006: Thread binding lives in the placement provider and fails closed

Status: Accepted

## Context

Moirai worker startup (`MOI-WORKER-CORE-PREMISE-2026-09-01`) must confine each
worker thread to one logical processor and publish the assignment only after
the binding took effect; a scheduler that cannot tell "bound" from "attempted"
reports a placement nobody enforces. Hermes's `ProcessorBinding` (one
processor) and `NumaBinding` (a node's processors) are scope guards that
restore the replaced affinity on drop, through their own
`SetThreadGroupAffinity` / `sched_{get,set}affinity` declarations; `NumaBinding`
is best-effort and reports what it bound (`NumaBindingCoverage`). Themis owns
the flattened numbering (`group * 64 + bit` on Windows) and the group masks
(`ProcessorGroupAffinity`, `ProcessorAffinityGroups`), so a second mechanism
re-derives that convention beside the call.

Two OS facts bound a set binding. Linux silently intersects the requested mask
with the CPUs present and permitted by cpusets ([sched_setaffinity(2)][linux],
NOTES). One `SetThreadGroupAffinity` call confines a thread to the one group it
names; on Windows 11 / Server 2022 hosts with more than 64 processors a thread
whose affinity was never set spans all groups, and `GetThreadGroupAffinity`
reports only its primary group ([SetThreadGroupAffinity][stga],
[GetThreadGroupAffinity][gtga], [Processor Groups][groups]).

## Decision

Themis exports, needing `alloc` but not the `std` feature:

- `bind_current_thread(processor: u32) -> Result<(), BindError>`: confine the
  calling thread to one processor for its life; no guard, a second call
  rebinds. It is the one-processor case of the set path below.
- `ThreadAffinity`: processors one platform call can express, with `Eq`,
  `contains(u32)`, `len()`, ascending `processors()`, and on Windows
  `From<ProcessorGroupAffinity>`. `from_processors` returns
  `SpansGroups { groups }` for a set touching several Windows groups and
  `NoActiveProcessor` for an empty set, so every value applies whole.
- `current_thread_affinity() -> Result<ThreadAffinity, BindError>`.
- `bind_current_thread_to(&ThreadAffinity) -> Result<AffinityChange,
  BindError>`; `AffinityChange::{previous, effective}` are the affinity
  replaced and the affinity read back after the call.

`Ok` means the kernel accepted the call; `effective` is the confinement in
force and may be a proper subset of the request, so a caller needing the exact
set compares the two. Every `Err` leaves the thread's affinity unchanged, with
one exception: a readback failing after the kernel accepted the call re-applies
`previous` before returning `Os`, and if that re-application also fails the
thread stays confined to what the kernel accepted. The documented errors of the
readback calls cannot occur after the same call succeeded on the same thread
with the same buffer.

`BindError` is `#[non_exhaustive]` and partitions failure by who can act:
`Unsupported` (no backend), `OutOfRange` (an id the affinity interface cannot
name: a Windows group beyond `u16`, a Linux id at or past the crate's
32 768-processor bound), `SpansGroups { groups: u32 }`, `NoActiveProcessor`
(empty set; on Windows a group the host lacks or a mask with no bit in its
`ActiveProcessorMask` ([PROCESSOR_GROUP_INFO][pgi])) and `Os { code }` (the
kernel refused: `GetLastError` or `errno`). All but `Os` precede any call.
Linux cannot see the cpuset beforehand, so an all-disallowed set there stays
the kernel's `Os { code: EINVAL }`.

**Restore stays with the caller.** Restoring is
`bind_current_thread_to(change.previous())`. It re-applies the snapshot
unconditionally, replacing any affinity set in between, and returns `Err`
without panicking when the snapshot is no longer allowed. Hermes keeps its
`!Send` guards, explicit `restore`, and best-effort `Drop` on top. The error
code is read through `GetLastError` / `__errno_location` (glibc and musl, as
hermes already declares at `numa/processor/linux.rs:48`), which removes the
`std::io::Error` read and its `expect`; `BindError` implements
`core::error::Error` (stable at the crate's `rust-version` 1.81, recalled).

**Windows.** The backend reads the group's active mask with
`GetLogicalProcessorInformationEx(RelationGroup)`, whose buffer is sized by
the `ERROR_INSUFFICIENT_BUFFER` protocol ([page][glpie]), so every bind,
restore included, allocates; allocation failure aborts, it does not panic.
It intersects, calls `SetThreadGroupAffinity` with the previous out-pointer,
and reads back with `GetThreadGroupAffinity`. The pages read do not say
whether `SetThreadGroupAffinity` rejects or intersects inactive bits
(`SetThreadAffinityMask` documents `ERROR_INVALID_PARAMETER` for a processor
outside the process mask ([SetThreadAffinityMask][stam])); pre-filter plus
readback holds under either. A `previous` from a never-bound thread on a
Windows 11 multi-group host names only its primary group, so restoring narrows
it to one group; that `PreviousGroupAffinity` follows the
`GetThreadGroupAffinity` rule is inferred, not stated.

**Linux.** The backend snapshots with `sched_getaffinity` into a fixed bitmap
of the 32 768-processor bound (4 KiB of `usize` words, the kernel's
`unsigned long` layout on every endianness), applies with `sched_setaffinity`,
and reads back into the same bitmap; nothing allocates.

**Other targets.** macOS, other Unix, wasm, and Miri return `Unsupported` from
every call, never a silent `Ok`. The entry points are declared in private
`extern` blocks, each with its `// SAFETY:` justification.

## Rejected alternatives

**Return `Result<Guard, _>` that restores on drop.** Moirai pins once. Hermes
needs a restore, but its guard policy (best-effort drop, narrowing-only
processor binding, coverage) is hermes's; a Themis guard would own a drop path
whose failure it cannot report, and a leaked guard would silently change what
the type promises. Returning `previous` gives the mechanism without the policy.

**Keep binding in hermes.** Hermes reports typed errors, but its mechanism
re-derives Themis's numbering and assumes active processors are the low
`GetActiveProcessorCount` bits, which no page read documents. Moirai already
consumes Themis; one mechanism belongs at the placement leaf.

**Bind the largest group of a multi-group set and report coverage.** `Ok` would
then mean "partly bound", which is policy inside the fail-closed primitive.
Hermes already selects through `ProcessorAffinityGroups::largest_group`, and
Themis detection reads one `GROUP_AFFINITY` per node
(`src/topology/cpu/detect/windows.rs:36,67`), so a reported node is one group.

**Windows CPU Sets.** `SetThreadSelectedCpuSetMasks` spans groups, but CPU Sets
are a documented "soft" affinity, ignored in groups not matching the thread's
affinity group, and need Windows 11 / Server 2022 ([CPU Sets][cpusets],
[SetThreadSelectedCpuSetMasks][stscsm]).

**One `SetThreadGroupAffinity` call per group** ends confined to the last.

**`Err` whenever `effective` differs.** On Linux the narrowing is in force
before it is visible, so the error path would need a second mutation that can
fail. Reporting `effective` states the fact without it.

**Stay `std`-only and gate hermes's guards on `std`.** Hermes compiles
`ProcessorBinding` without `std` and CI checks that build (hermes
`.github/workflows/ci.yml:110`); gating would turn it into
`UnsupportedPlatform`.

**Add `libc` / `windows-sys`, or report `Os` as a string.** A dependency edge
adds lockfile and MSRV surface to every consumer; consumers branch on codes.

## Consequences

Additive `[minor]` surface: three functions, two types, two error variants.
On Windows, `bind_current_thread` now reports an absent group or inactive
processor as `NoActiveProcessor` before the call instead of the kernel's code;
Moirai's `non_exhaustive` arm maps unknown variants to `Unsupported`
(`moirai-executor/src/schedule/runtime/scheduler/placement.rs:179-182`), so it
adds an arm when it adopts this revision. Hermes moves both guards onto these
calls and drops its affinity declarations and active-mask assumption
(`HERMES-NUMA-BINDING-ON-THEMIS-2026-09-29`); libnuma's `numa_bind`, which also
sets memory policy, is outside this decision.

Overturning evidence: a Windows 11 multi-group consumer needing a faithful
restore of a never-bound thread; a documented `SetThreadGroupAffinity` rule for
inactive bits; or a target whose affinity interface cannot name a `u32`. The
backends run on the CI ubuntu and Windows legs; Miri reports `Unsupported`.

Revision 2026-11-13: extend to processor sets with a caller-held restore
snapshot, because hermes's guards need both
(`HERMES-NUMA-BINDING-ON-THEMIS-2026-09-29`). Evidence: hermes
`numa/{processor,binding,affinity}.rs` and themis `src/topology/cpu/` at their
2026-11-13 default branches, and the OS pages linked below, read that day.

[linux]: https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html
[stga]: https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-setthreadgroupaffinity
[gtga]: https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-getthreadgroupaffinity
[groups]: https://learn.microsoft.com/en-us/windows/win32/procthread/processor-groups
[stam]: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadaffinitymask
[pgi]: https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_group_info
[glpie]: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex
[cpusets]: https://learn.microsoft.com/en-us/windows/win32/procthread/cpu-sets
[stscsm]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadselectedcpusetmasks
