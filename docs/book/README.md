# themis — Typed Placement Law for Atlas

`themis` owns the **placement vocabulary** of the Atlas stack: NUMA node
identities, memory-tier classifications, locality domain IDs, worker IDs, and
the placement hint enum that ties them together.  It does not allocate, and it
does not execute — it only defines the types that `mnemosyne` and `moirai`
consume to route memory and compute to the right hardware.

## Design goals

- **Pure vocabulary** — no allocation, no I/O, no system calls in the core
  law types.  The vocabulary compiles to `no_std`.
- **Unknowability is type-level** — device capacities that a driver did not
  report are `None`; sentinel zeros are not representable.
- **Const-generic bucket tables** — `NumaBucketIndex<N>` fixes the bucket
  count at compile time, making NUMA-indexed tables bounds-free at runtime.
- **Branded placement** — with the `melinoe` feature, placement tokens carry
  a lifetime brand that prevents them from outliving the topology snapshot
  that created them.

## What this book covers

1. Placement hints: how callers express locality preferences without knowing
   the underlying topology.
2. Memory tiers: the allocatable vs. budgeted distinction.
3. Locality identities: NUMA node IDs, bucket indices, worker IDs.
4. CPU, GPU, and TPU topology snapshots.
5. Where themis sits in the Atlas stack and how mnemosyne and moirai use it.
