# ADR 0005: SMT siblings are a reported partition, and absence is typed

Status: Accepted

## Context

`CpuTopology` reported NUMA nodes, cache levels, logical processor count, node
distances, and (since ADR 0004) efficiency classes. It did not report which
logical processors share a physical core. Consumers that need that answer
guessed: a compute-bound worker pool sized by `logical_processors` treats two
SMT siblings as two cores and oversubscribes by 2x, and a measurement
instrument that pins to "the next free processor" can land on the sibling of a
busy core and measure contention rather than the code under test. The pinned
probes in apollo and mnemosyne are the present consumers.

The platform knows. Windows returns the core grouping in the same
`RelationProcessorCore` records the efficiency backend already walks: each
record is one core and its `GROUP_AFFINITY` names every logical processor on
it. Linux publishes `thread_siblings_list` per processor. Neither is derivable
from core counts or model strings.

## Decision

`CpuTopology` gains a per-processor core table, `CoreId`, a dense ordinal over
`u32` assigned in ascending order of each core's lowest logical processor.
Presence is discharged once through `CpuTopology::smt`, whose `CpuSmtView`
answers the placement questions directly: the core of a processor, whether two
processors are siblings, the processors of a core, and one processor per core
for instruments and pools.

Three properties carry the decision, and all three are ADR 0004's, applied to
a second axis:

**A partition, not a flag.** "Has SMT" is a boolean projection that says
nothing about which processors pair. The table says exactly that, and the
boolean falls out of it (`processors > cores`).

**Dense ids by first appearance.** Windows record order and Linux sibling-list
contents are both platform detail; the table compresses them to `0..cores` in
processor order, so a consumer walking processors meets core 0 first and
`one_processor_per_core` is a single pass.

**Absence is typed.** A host without SMT reports a present table with one
processor per core; that is a fact. `None` is reserved for a platform that
reported nothing usable, a table that does not cover every processor, a
processor claimed by two cores, or an asymmetric sibling list. A partial table
would fabricate "distinct cores" over the processors it missed, which is the
oversubscription this axis exists to prevent.

The Windows backend reuses the efficiency axis's record walk (`records.rs`)
rather than issuing a second `GetLogicalProcessorInformationEx` walk with its
own parser: one walk, one set of malformed-buffer rules, one set of fixtures,
two axes read from it.

## Consequences

Additive: a new field, a new type, a new view, a new accessor; `new_for_test`
keeps its signature and `with_core_ids_for_test` is the companion.
`ProcessorClass` (crate-private) carries a core ordinal the efficiency axis
ignores.

Axes 2 and 3 of `THEMIS-PLACEMENT-AXES-2026-09-01`, favoured cores within a
class and last-level-cache domains, are not built here. Each follows this
shape when a consumer needs it, per the item's sequencing rule.
