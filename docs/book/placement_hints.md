# 1. Placement Hints

`PlacementHint` is the single type that callers use to express a locality
preference to downstream consumers — chiefly the mnemosyne allocator and the
moirai scheduler.  It lives in `src/law/placement.rs` and is defined as:

```rust,ignore
pub enum PlacementHint {
    #[default] Current,
    Numa(NumaNodeId),
    Domain(LocalityDomainId),
    Tier(MemoryTier),
    Any,
}
```

## Variant semantics

**`Current`** (the default) tells the consumer to use the caller's current
NUMA node.  In the common case where a thread is already pinned or has warm
TLB entries on a particular node, this is the lowest-latency choice and
requires no topology lookup on the caller's side.

**`Numa(NumaNodeId)`** names a specific NUMA node.  The caller must hold a
valid [`NumaNodeId`](locality_identities.md) obtained from a
[`CpuTopology`](cpu_topology.md) snapshot; passing `NumaNodeId::INVALID`
is not a logic error (the type does not enforce validity), but consumers will
treat an invalid id as a hint they cannot satisfy and fall back to `Current`.

**`Domain(LocalityDomainId)`** names a locality domain — a coarser grouping
than a single NUMA node.  A locality domain typically corresponds to one CPU
socket and may span multiple NUMA nodes that share the same memory-distance
law.  Using `Domain` is appropriate when the caller cares about socket-local
access but does not need to pin to a specific sub-node.

**`Tier(MemoryTier)`** expresses a technology preference rather than a
topological one.  A caller that needs high-bandwidth memory writes
`Tier(MemoryTier::Hbm)` without knowing which NUMA node or device holds that
memory.  The allocator resolves the tier against the detected
[`CpuTopology`](cpu_topology.md) or [`GpuTopology`](gpu_topology.md) to find
a suitable region.  Note that `Tier(MemoryTier::Registers)` and
`Tier(MemoryTier::SharedMem)` are budget-only tiers;
[`MemoryTier::is_host_allocatable`](memory_tiers.md) returns `false` for both,
and a well-behaved allocator rejects such hints rather than silently ignoring
them.

**`Any`** tells the consumer that the caller has no locality preference.  The
allocator is free to choose the cheapest available region.  Use `Any` only
when the allocation is genuinely topology-indifferent (e.g. a small
scratch buffer whose lifetime is shorter than a cache miss).

## Properties

`PlacementHint` is `Copy + Hash`.  It is safe to store in a `HashMap`, embed
in task descriptors, or pass across thread boundaries without cloning.  The
entire enum fits in one word on all supported targets.

## Preference, not mandate

A `PlacementHint` is a request, not a binding constraint.  If the preferred
NUMA node is full or the requested tier is unavailable, the allocator falls
back gracefully — typically to `Current`, then to `Any`.  Callers that
require hard placement guarantees must coordinate with the platform's memory
policy layer (outside themis scope).

## Consumers

- **mnemosyne** matches on `PlacementHint` to select its NUMA policy and
  memory backend before calling into the OS allocator.
- **moirai** uses `Numa(id)` and `Domain(id)` to pin tasks to workers or
  work-stealing domains.

See the [worked example](examples/placement_hints.md) for a pattern-match
dispatch that mirrors what a real allocator would do.
