# ADR 0006: Thread binding lives in the placement provider and fails closed

Status: Accepted

## Context

Moirai worker startup (`MOI-WORKER-CORE-PREMISE-2026-09-01`) must confine each
worker thread to one logical processor and publish the worker-to-processor
assignment only after the binding took effect. A scheduler that cannot tell
"bound" from "attempted" reports an assignment nobody enforces, which is the
fabricated placement fact the crate exists to prevent.

The only binding code in the stack is hermes's `NumaBinding`, which binds a
thread to a NUMA node's processors and never reports failure: it is best-effort
by contract, correct for a locality hint and unusable for a scheduler that must
refuse to start. Themis already owns the flattened processor numbering
(`group * 64 + bit` on Windows) and the group-mask representation
(`ProcessorGroupAffinity`), so a consumer binding a thread would otherwise
re-derive that convention beside the call, the duplication `THEMIS-AFFINITY-MASK-ACCESSOR-2026-09-01` was filed to end.

## Decision

Themis exports `bind_current_thread(processor: u32) -> Result<(), BindError>`
(`std` feature). It confines the calling thread to exactly one logical
processor for the rest of the thread's life; a second call rebinds. There is no
guard and no restore, because a worker pins once at startup.

`BindError` is `#[non_exhaustive]` and partitions failure by who can act:
`Unsupported` (no backend: macOS, other Unix, wasm, or Miri), `OutOfRange`
(the id cannot be named by the target's affinity interface), and `Os { code }`
(the kernel refused; `GetLastError` or `errno`). `Ok(())` is returned only when
the kernel accepted the call, and every error leaves the thread's affinity
unchanged. A target without a backend is never a silent `Ok`.

Windows maps the id through `ProcessorGroupAffinity::from_processor` and calls
`SetThreadGroupAffinity` on the current thread. Linux builds a one-bit mask
sized to the processor and calls `sched_setaffinity(0, ..)`. The mask is a
`usize` bitmap, matching the kernel's `unsigned long` words on every
endianness, rather than a fixed 1024-bit `cpu_set_t`. Ids at or past the crate's
existing processor-id bound (32 768) are `OutOfRange` before any allocation, so
a caller-supplied `u32` cannot size a 512 MiB mask. A processor that is offline
or outside the process's cpuset reaches the kernel and returns `EINVAL`, which
is the honest report.

The three kernel entry points are declared in private `extern` blocks, as the
detection backends already do. The crate declares no `forbid(unsafe_code)`;
each block carries its `// SAFETY:` justification.

## Rejected alternatives

**Add `libc` / `windows-sys`.** Three functions and one `#[repr(C)]` struct do
not justify a dependency edge in the leaf of the placement stack. The crate
already declares its Win32 and glibc entry points inline, and a new edge would
add lockfile and MSRV surface to every consumer for no shared code.

**Keep binding in hermes.** Hermes's contract is best-effort node binding, and
its consumers cannot fail on it. Moirai already consumes Themis for placement
law; taking a fail-closed processor primitive from a SIMD crate instead would
add a dependency edge for one function and leave the numbering convention
encoded outside Themis a second time.

**Return `Result<Guard, _>` that restores the previous affinity.** Workers pin
once and never unpin; a guard would add a lifetime and a restore path no
consumer uses, and a leaked guard would silently change what the type promises.

**Report `Os` as a string.** The consumer branches on the code (retry on
another processor, abort startup); a string forces it to parse.

## Consequences

Additive `[minor]` surface: one function and one error type. Moirai can refuse
to start on `Err` and publish assignments after `Ok`.

Hermes may later express `NumaBinding` on this primitive, keeping its
best-effort policy in hermes and its mechanism here. That migration is filed as
`HERMES-NUMA-BINDING-ON-THEMIS-2026-09-29` on the Themis board and is not part
of this decision.

Overturning evidence: a consumer that needs to bind a thread to a processor
*set* (not one processor) with a typed failure would extend this primitive
rather than replace it; a target whose affinity interface cannot name a
processor by a `u32` would add an `OutOfRange` case, not a new variant.

The Linux backend is exercised by the CI ubuntu leg; the Windows backend by the
Windows leg. Neither backend is exercised under Miri, where binding reports
`Unsupported`.
