# ADR 0004: Efficiency class is a dense ordinal, and absence is typed

Status: Accepted

## Context

`CpuTopology` modelled NUMA nodes, cache levels, logical processor count, and
node distances, but not the performance/efficiency distinction on hybrid CPUs.
Consumers that need it hand-rolled machine-specific constants instead: apollo's
pinned probes encoded "logical 0..8 are the performance cores" and pinned to
cpu 2 and cpu 12; mnemosyne's benchmark harness parsed its own Windows
performance-core mask.

Both apollo labels were wrong on the host they were written for. That host
reports its performance cores as the mask `0xc03c03` — processors
`{0, 1, 10, 11, 12, 13, 22, 23}` out of 24 — so cpu 2 is not a performance core,
and a probe pinned there measures one of the slowest processors on the part. A
merged ADR drew an inverted conclusion from those numbers. The performance-core
set is not a contiguous low range, is not derivable from core counts or model
strings, and is knowable only by asking the platform.

## Decision

`CpuTopology` gains an efficiency-class table indexed by processor id, with
`EfficiencyClass` as a dense ordinal newtype over `u8`, higher meaning more
performant.

Three properties carry the decision:

**Ordinal, not boolean.** Windows reports a `PROCESSOR_RELATIONSHIP`
`EfficiencyClass` byte and Linux ARM reports a `cpu_capacity` scalar; both are
ranks, and parts already ship with performance, efficient, and low-power
efficient tiers. A `bool is_performance_core` would be a lossy projection that a
three-tier part breaks.

**Dense ranks.** Each backend reduces its platform data to one raw capability
value per processor and the shared ranker compresses those to `0..n`, so a
Windows byte pair of `{0, 2}`, an ARM capacity pair of `{462, 1024}`, and Intel
type-set membership all land on the same comparable type. Ranks are therefore
meaningful only within one snapshot, not as a cross-machine scale.

**Absence is typed and total.** `None` means the platform did not report, and is
never a fabricated two-class split — the same discipline `cache_levels()`
already carries. Coverage is all-or-nothing: a table is published only when it
covers exactly `0..logical_processors`, because a table with holes would be a
fabricated split over the processors it did cover. `efficiency_class_count()`
is the absence oracle: `None` is "not reported", `Some(1)` is a homogeneous
host, `Some(n > 1)` is hybrid. Homogeneous hosts — most servers, most CI
runners — are a reported result, not an edge case, and are distinguishable from
absence at every accessor.

Multi-group Windows hosts are supported by reading each record's explicit group
index into this crate's `group * 64 + bit` numbering, with the coverage rule as
the safety net: the existing Windows NUMA backend reads a single
`GROUP_AFFINITY` per node, so on a host where the two APIs enumerate groups
differently the coverage check fails and the result is absence rather than a
class table keyed by ids the rest of the snapshot does not use.

## Alternatives

A `bool` performance-core flag matches what today's consumers ask for and is a
smaller surface, but it cannot represent a third tier and would need a breaking
change to gain one; the ordinal answers the same question through
`highest_efficiency_class()` at no extra cost to the caller.

Publishing a partial table where the platform covers only some processors would
raise availability, but a consumer cannot distinguish "processor 40 is class 0"
from "processor 40 was not reported", which is exactly the failure this record
exists to prevent.

Falling back to a heuristic — core-count ratios, model-string matching, or
timing probes — would make the accessor always answer. It would also reproduce
the incident: the wrong answer is what the hand-rolled constants already
provided, and a wrong answer inside the topology crate is worse than one in a
consumer, because every consumer inherits it.

Reporting absence on every multi-group host was considered as a simpler rule
than the coverage check. It withholds correct data from large hosts that
enumerate consistently, and the coverage check already yields absence in exactly
the cases where the numbering would disagree.

## Verification

The parsers are pure functions over recorded platform bytes and sysfs strings,
compiled on every target so each CI leg exercises both backends' parsing.
Fixtures pin the recorded 24-processor `0xc03c03` host end to end and assert
that a consumer asking for a performance processor never receives cpu 2; a
homogeneous host; a three-tier host; sparse class bytes; a fully and a partially
enumerated multi-group host; records disagreeing about one processor; and
malformed, truncated, empty, and relationless buffers. A host-dependent smoke
test asserts the published invariants on whatever the running machine reports
and skips with a printed reason where nothing is reported, so an unreporting
host never passes silently. The developer host's live detection returns exactly
`{0, 1, 10, 11, 12, 13, 22, 23}`, matching the recorded mask.

Format, warning-denied Clippy across both feature configurations, nextest, and
doctests are the acceptance gates. The change is additive: no existing signature
or behavior changes, so it is `[minor]`.
