# Themis

Themis provides typed placement law for Atlas runtime and memory crates.

It is the shared source of truth for:

- NUMA node identity
- worker identity
- locality domains
- memory tiers, including HBM
- placement hints
- topology snapshots
- current CPU/NUMA node queries

It does not own allocation, scheduling, queues, worker loops, or thread-local
allocator state. Mnemosyne remains the allocation owner. Moirai remains the
execution owner. Themis supplies the law both consume.

With the default `melinoe` feature, Themis also exposes branded placement
scopes. `ThreadLocalPlacement` uses Melinoe's thread-confined token for
worker-local placement state. `SyncRegionPlacement` uses Melinoe's sync-region
token for placement snapshots that may move between execution domains. Branded
storage remains `melinoe::MelinoeCell`; Themis does not define a second cell
name.

## Boundary

```text
themis
├── typed placement identifiers
├── memory tier and placement hint vocabulary
├── topology snapshots and distance lookup
└── current locality query

mnemosyne -> themis   allocation placement
moirai    -> themis   worker and task placement
leto      -> themis   cache-sized tiling hints
```

No Themis API stores allocator state, scheduler state, or raw thread-local
storage pointers.

`CacheLevel` is topology law, not a cache detector. Leto consumes cache sizes
as tiling hints, and Moirai consumes shared-processor rows as chunk-locality
hints. Unknown cache properties stay represented by conservative provider
defaults until a platform backend supplies stronger data.

## Evidence

Current correctness claims rest on type-level encoding plus value-semantic unit
tests. Branded placement-state claims rest on Melinoe token invariants. OS
topology discovery is empirical and falls back to a single-node topology when
platform data is unavailable. Cache consumer contracts are documentation-level
evidence until leto and moirai add contract tests against the public surface.
