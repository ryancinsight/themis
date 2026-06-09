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

## Boundary

```text
themis
├── typed placement identifiers
├── memory tier and placement hint vocabulary
├── topology snapshots and distance lookup
└── current locality query

mnemosyne -> themis   allocation placement
moirai    -> themis   worker and task placement
```

No Themis API stores allocator state, scheduler state, or raw thread-local
storage pointers.

## Evidence

Current correctness claims rest on type-level encoding plus value-semantic unit
tests. OS topology discovery is empirical and falls back to a single-node
topology when platform data is unavailable.

