# Themis

Themis provides typed placement law for Atlas runtime and memory crates.

The crates.io package is named `themis-topology`; its Rust library crate
remains `themis` so existing imports do not change.

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

### Current locality query contract

`current_numa_node()` is the fast placement path: under `std` it caches the
calling thread's last observed NUMA node in thread-local storage and falls back
to `NumaNodeId::ZERO` when the platform cannot report locality. It does not
re-probe the operating system on every call. Callers that may migrate between
NUMA nodes must call `refresh_current_numa_node()` at an explicit scheduling
boundary; the refresh probes the platform and replaces that thread's cached
value. `try_current_numa_node()` and `current_processor()` are uncached,
optional probes for verification or diagnostics and preserve `None` when the
platform does not expose the requested value. In `no_std` builds the cached
placement query has the deterministic node-zero fallback because no OS probe is
available.

This distinction is intentional: Themis owns the locality vocabulary and query
contract, while Moirai decides scheduling boundaries and Mnemosyne decides how
a placement hint affects allocation. No consumer should add a second NUMA
cache or substitute a fabricated cache/topology value.

`CacheLevel` is provider-reported topology law. Linux reads cache-index records
from sysfs and Windows reads `GetLogicalProcessorInformationEx`; unavailable or
malformed cache data is `None`, never a synthetic capacity. Leto consumes cache
sizes as tiling hints, and Moirai consumes shared-processor rows as
chunk-locality hints. Consumers must preserve typed cache absence.

## Benchmarks

Run the topology benchmark with:

```text
cargo bench --bench topology
```

The benchmark exercises real `CpuTopology::single_node` construction and
`processor_node_pairs` traversal. Its output is empirical local timing only;
it is not a statistical baseline or a speedup claim.

## Evidence

Current correctness claims rest on type-level encoding plus value-semantic unit
tests. Branded placement-state claims rest on Melinoe token invariants. OS
topology discovery is empirical and falls back to a single-node topology when
platform data is unavailable. Cache discovery is empirical platform data with
value-semantic parser tests; cache absence remains explicit until a provider
reports a complete hierarchy. Leto and Moirai carry the typed absence through
their public consumer surfaces.
Benchmark claims currently rest on the dependency-free topology benchmark
harness and must be treated as empirical local timing unless a criterion
baseline is added.
