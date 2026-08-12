# 8. Position in the Stack

Themis occupies the placement-vocabulary layer of the Atlas stack.  It defines
the types that express *where* and *on what* work should run, without owning
the allocation or scheduling logic that acts on those expressions.

## Atlas layer ordering

```
eunomia  →  aequitas  →  themis
                          ↓           ↓
                       mnemosyne    moirai
```

`eunomia` provides foundational numeric and error-handling primitives.
`aequitas` builds fairness and scheduling-policy abstractions on top.
`themis` sits at the same level as `aequitas` in the dependency graph and
provides the typed placement vocabulary that both `mnemosyne` (allocator) and
`moirai` (scheduler) consume.

## What themis owns

- **Placement vocabulary**: [`PlacementHint`](placement_hints.md),
  [`MemoryTier`](memory_tiers.md) and `MemoryTier::is_host_allocatable`.
- **Identity newtypes**: [`NumaNodeId`](locality_identities.md),
  [`LocalityDomainId`](locality_identities.md),
  [`WorkerId`](locality_identities.md),
  [`NumaBucketIndex<N>`](locality_identities.md).
- **Topology snapshots**: [`CpuTopology`](cpu_topology.md),
  [`GpuTopology`](gpu_topology.md) / `GpuDeviceProperties`,
  [`TpuTopology`](tpu_topology.md) / `TpuDeviceProperties`.
- **Invalidation token**: [`TopologyEpoch`](topology_epoch.md).

## themis → mnemosyne

`PlacementHint` is the primary interface between a caller and the allocator:

- `PlacementHint::Current` / `Numa(id)` / `Domain(id)` select the NUMA policy
  (which node's memory pool to allocate from, and what fallback order to use
  if that pool is full).
- `PlacementHint::Tier(MemoryTier)` selects the memory backend: DRAM vs. HBM
  vs. host-pinned, resolved against the topology snapshot.
- `MemoryTier::is_host_allocatable()` gates allocation: mnemosyne rejects
  hints that name non-allocatable tiers (`Registers`, `SharedMem`) rather than
  silently ignoring them.

## themis → moirai

- `WorkerId` drives per-worker affinity settings; moirai assigns workers from
  its pool and stores their ids alongside their OS thread handles.
- `NumaNodeId` drives task routing: when a task carries
  `PlacementHint::Numa(id)`, moirai picks the worker whose `NumaNodeId`
  matches.
- `CpuTopology` drives work-stealing domain boundaries: moirai groups workers
  that share an L3 cache (via `CacheLevel::shared_processors`) into the same
  steal domain, so intra-domain stealing is attempted before crossing a NUMA
  or socket boundary.

## hephaestus → themis types

Hephaestus is the device-backend crate.  It reads wgpu adapter limits or CUDA
device attributes and constructs `GpuDeviceProperties` values, then hands
the assembled `GpuTopology` snapshot to moirai's occupancy planner.  Themis
provides no `detect()` equivalent for GPUs — the types are populated by the
backend, not by themis itself.

## What themis does NOT own

- **Allocation** — themis defines where memory should go; mnemosyne decides
  how to obtain it (system calls, pool allocation, fallback chains).
- **Scheduling** — themis defines worker and domain identity; moirai decides
  when a task runs and on which worker.
- **Branded capability evidence** — melinoe (and its `halo` sub-crate) owns
  the type-level proof tokens that certify a thread holds a particular
  capability.  Themis identity types are plain IDs, not capability tokens.
