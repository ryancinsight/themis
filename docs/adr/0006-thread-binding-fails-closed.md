# ADR 0006: Thread binding lives in the placement provider and fails closed

Status: Accepted

## Context

Moirai publishes a worker's processor only after the binding took effect
(`MOI-WORKER-CORE-PREMISE-2026-09-01`). Hermes's `ProcessorBinding` and
`NumaBinding` guards restore the replaced affinity on drop, and
`ProcessorIndex::current` reports the running processor, through hermes's own
affinity and current-processor declarations; hermes ADR 021 keeps
`ProcessorBinding` allocation-free and available without `std`. Themis owns
the flattened numbering (`group * 64 + bit` on Windows) and the group masks.

Linux silently intersects a requested mask with the CPUs present and permitted
by cpusets ([sched_setaffinity(2)][linux], NOTES). `SetThreadGroupAffinity`
confines a thread to the one group it names; on Windows 11 / Server 2022 hosts
with more than 64 processors a never-bound thread spans all groups and
`GetThreadGroupAffinity` reports only its primary group ([SetThreadGroupAffinity][stga],
[GetThreadGroupAffinity][gtga], [Processor Groups][groups]).

## Decision

Themis exports, without the `std` feature and without allocating:

- `ThreadAffinity`: processors one platform call can express (`Eq`,
  `contains`, `len`, ascending `processors`), validated at construction by
  `from_processors` and, on Windows, `from_group(ProcessorGroupAffinity)`.
- `current_thread_affinity() -> Result<ThreadAffinity, BindError>`, holding
  only the primary group for a never-bound Windows 11 multi-group thread.
- `bind_current_thread_to(&ThreadAffinity) -> Result<AffinityChange, BindError>`;
  `AffinityChange::{previous, effective}` are the affinity replaced and the
  affinity read back. `effective` may be a proper subset of the request; an
  `Err` for that case would need a second, fallible mutation on Linux.
- `bind_current_thread(processor)`: `from_processors([processor])`, then bind.
- `current_processor()`, reporting without `std` and independently of the node
  lookup and its bound.

Every `Err` leaves the affinity unchanged, except that a readback failing after
the kernel accepted the call re-applies `previous` before returning `Os` (if
that fails too, the accepted affinity stays). On Linux the readback repeats the
snapshot call, whose documented errors (`EFAULT`, `ESRCH`, `EINVAL` for a short
mask) the snapshot excluded; on Windows `GetThreadGroupAffinity` first runs at
the readback and names no specific error, so the exception is reachable there.

`BindError` is `#[non_exhaustive]`: `Unsupported` (no backend), `OutOfRange`
(a Windows group beyond `u16`, a Linux id at or past the crate's
32 768-processor bound), `SpansGroups { groups: u32 }` (several Windows groups;
one call confines to one group, and partial binding is hermes's coverage
policy), `NoActiveProcessor` (an empty set; on Windows a group the host lacks
or a mask with no bit in its `ActiveProcessorMask` ([PROCESSOR_GROUP_INFO][pgi])),
and `Os { code: i32 }` (`GetLastError` or `errno`). All but `Os` arise at
construction, before any mutation; the Windows active-group query can itself
fail as `Os`. An all-disallowed Linux set stays the kernel's
`Os { code: EINVAL }`. `BindError` implements `core::error::Error`, stable since
Rust 1.81 ([core::error::Error][error]), the crate's `rust-version`.

**Restore stays with the caller and cannot allocate.** Restoring is
`bind_current_thread_to(change.previous())`, which replaces any affinity set in
between and returns `Err`, never panicking, when the kernel refuses it. The set
call runs no validation query: construction validated a caller-built value and
the kernel produced `previous`. Hermes keeps its `!Send` guards, explicit
`restore` and best-effort `Drop`. No path reaches `handle_alloc_error`, which
by default aborts when `std` is linked ("Future versions of Rust may panic by
default instead") and calls `panic!` when it is not ([handle_alloc_error][hae]),
so hermes's `Drop` stays panic-free without `std` and ADR 021 holds. The token
grep in the oracle of `HERMES-NUMA-BINDING-ON-THEMIS-2026-09-29` checks it; the
no-`std` build does not reject an allocating import, because the topology
tables link `alloc` there.

**Windows.** Construction applies the host-free checks (`OutOfRange`,
`SpansGroups`, empty set) first, then the binding backend narrows the mask to
the group's active bits (`NoActiveProcessor` when none remain), read by
`GetLogicalProcessorInformationEx(RelationGroup)` into a stack buffer for 512
groups, the 32 768-processor bound: an 8-byte record header, the 24-byte
`GROUP_RELATIONSHIP` prefix and one 48-byte `PROCESSOR_GROUP_INFO` per active
group ([GROUP_RELATIONSHIP][gr]), 24 608 bytes (derived for a 64-bit
`KAFFINITY`; at 4 bytes the entry is 44 and `GROUP_AFFINITY` 12, and Themis
configures no 32-bit Windows target). A larger reply
fails with `ERROR_INSUFFICIENT_BUFFER` ([page][glpie], ReturnedLength) as `Os`,
never read truncated. `ThreadAffinity` is one 16-byte `GROUP_AFFINITY`, applied
by `SetThreadGroupAffinity` with the previous out-pointer and read back by
`GetThreadGroupAffinity`. No page read says whether that call rejects or
intersects inactive bits (`SetThreadAffinityMask` documents
`ERROR_INVALID_PARAMETER` outside the process mask ([page][stam])); narrowing
plus readback holds under either. A never-bound Windows 11 multi-group thread's
`previous` names only its primary group, so restoring narrows it to one group
(inferred from the `GetThreadGroupAffinity` rule). `GetCurrentProcessorNumberEx`
has no failure return ([page][gcpne]).

**Linux.** `ThreadAffinity` is an inline 4 KiB bitmap of `c_ulong` words, the
kernel's `unsigned long` mask layout ([sched_setaffinity(2)][linux], C
library/kernel differences): `AffinityChange` is 8 KiB, a guard keeping
`previous` 4 KiB (hermes's `CpuSet` is 128 bytes). `sched_getaffinity`,
`sched_setaffinity` and `__errno_location` come from the `libc` 0.2 edge Themis
already carries for `getcpu`, the bitmap passed by pointer cast with
`cpusetsize` 4096, which "is the length (in bytes) of the data pointed to by
mask", so libc's 128-byte `cpu_set_t` pointee bounds nothing. `getcpu(2)`
documents only `EFAULT` ([page][getcpu]).

Other targets (macOS, other Unix, wasm, Miri) return `Unsupported`, never a
silent `Ok`, and `current_processor` returns `None`.

## Rejected alternatives

**Return a guard that restores on drop.** The guard policy is hermes's; a
Themis guard would own a drop path whose failure it cannot report.

**Keep binding in hermes.** It re-derives Themis's numbering and assumes active
processors are the low `GetActiveProcessorCount` bits, which no page read
documents; one mechanism belongs at the placement leaf Moirai already uses.

**Stay `std`-only and gate hermes's guards on `std`.** `ProcessorBinding` and
`ProcessorIndex::current` work without `std` in hermes today (own FFI), so
gating them makes both `UnsupportedPlatform`; `NumaBinding::bind` returns
`Self`, its Windows arm is `cfg(all(target_os = "windows", feature = "std"))`
(`numa/binding.rs:96`) and the fallback is `Self::unbound()` (`:106-113`), so it
stays `Unbound` there. Hermes's `no_std` checks (`ci.yml:110`, `:370`) run
`--workspace`, which unifies `std` on through `hermes-simd-intrinsics` and
`hermes-simd-benches`, so the item names the package-scoped check instead. That
check proves only that the no-`std` build compiles: gated guards compile as
`UnsupportedPlatform` stubs, so it passes either way, and the
`hermes-simd-intrinsics` edges link `std` into tests.

**Leave `ProcessorIndex::current` on hermes's own FFI.** It keeps working, but
hermes would keep private `sched_getcpu` / `GetCurrentProcessorNumberEx`
declarations and its own `group * 64 + number` flattening for a query Themis
already exports, so the mechanism stays duplicated outside its owner.

**Validate inside every set call.** Restore would repeat the active-group
query, and a heap reply buffer would put `handle_alloc_error` on `Drop`.

**Windows CPU Sets** span groups but are a documented 'soft' affinity, ignored
in groups not matching the thread's affinity group, and need Windows 11 /
Server 2022 ([CPU Sets][cpusets], [SetThreadSelectedCpuSetMasks][stscsm]).

**Add `windows-sys`, or report `Os` as a string.** Themis's normal Windows
graph is melinoe alone (`cargo tree -e normal`); `windows-sys` would reach every
Windows consumer to replace declarations Themis writes. Consumers branch on codes.

## Consequences

Additive `[minor]` surface: `current_thread_affinity`, `bind_current_thread_to`,
`ThreadAffinity` (`from_processors`, Windows `from_group`), `AffinityChange`,
`SpansGroups`, `NoActiveProcessor`; `bind_current_thread` and `BindError` lose
their `std` gate. Without `std`, `current_processor`, `try_current_numa_node`
and `refresh_current_numa_node` report the OS value instead of `None` / node 0;
uncached `current_numa_node` stays node 0. Windows `bind_current_thread` reports
an absent group or inactive processor as `NoActiveProcessor`, not the kernel's
code; Moirai maps unknown variants to `Unsupported`
(`moirai-executor/src/schedule/runtime/scheduler/placement.rs:179-182`) and
adds an arm on adoption. Hermes moves both guards and `ProcessorIndex::current`
onto these calls, mapping `None` to `UnsupportedPlatform` (`sched_getcpu`
documents only `ENOSYS` ([page][getcpu3])), revises ADR 021 option 2 and drops
its affinity declarations (`HERMES-NUMA-BINDING-ON-THEMIS-2026-09-29`);
libnuma's `numa_bind`, which also sets memory policy, stays outside.

Overturning evidence: a consumer needing a faithful restore of a never-bound
Windows 11 multi-group thread; a documented `SetThreadGroupAffinity` rule for
inactive bits; a host past 512 active groups; an id beyond `u32`.

Revision 2026-11-13: extend to processor sets with an allocation-free, caller-held restore and a `no_std` `current_processor` for hermes's guards (`HERMES-NUMA-BINDING-ON-THEMIS-2026-09-29`); evidence: hermes and themis default branches and the linked pages, read 2026-11-13; the std-gating alternative corrected against hermes `numa/` and `cargo tree`.

[linux]: https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html
[getcpu]: https://man7.org/linux/man-pages/man2/getcpu.2.html
[getcpu3]: https://man7.org/linux/man-pages/man3/sched_getcpu.3.html
[stga]: https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-setthreadgroupaffinity
[gtga]: https://learn.microsoft.com/en-us/windows/win32/api/processtopologyapi/nf-processtopologyapi-getthreadgroupaffinity
[groups]: https://learn.microsoft.com/en-us/windows/win32/procthread/processor-groups
[stam]: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-setthreadaffinitymask
[pgi]: https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-processor_group_info
[gr]: https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-group_relationship
[glpie]: https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex
[gcpne]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocessornumberex
[hae]: https://doc.rust-lang.org/stable/alloc/alloc/fn.handle_alloc_error.html
[error]: https://doc.rust-lang.org/stable/core/error/trait.Error.html
[cpusets]: https://learn.microsoft.com/en-us/windows/win32/procthread/cpu-sets
[stscsm]: https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadselectedcpusetmasks
