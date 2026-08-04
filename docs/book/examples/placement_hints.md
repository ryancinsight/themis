# Example: Placement Hints

**Crate**: `themis`
**Source**: `examples/book_placement_hints.rs`

The `PlacementHint` enum is the single type that allocation callers and
scheduling callers use to express locality preferences without knowing the
underlying topology.  This example constructs every hint variant, matches on
them to show how an allocator would dispatch, maps NUMA node IDs into a
fixed-size bucket table, and confirms the host-allocatability contract for
all eight memory tiers.

## Source

```rust
{{#include ../../../examples/book_placement_hints.rs}}
```

## Output

```text
default: allocate on the caller's current NUMA node
NUMA(NumaNodeId(2)): allocate on the specified NUMA node
Domain(LocalityDomainId(0)): allocate within the specified locality domain
Tier(Hbm): allocate from the specified host-allocatable tier
Tier(Registers): invalid for allocation — budgeted tier only
host-allocatable tiers: [Dram, Hbm, Gddr, HostPinned, Device, Persistent]
node 0 → bucket 0
node 1 → bucket 1
node 2 → bucket 2
node 3 → bucket 3
node 4 → bucket 0
node 5 → bucket 1
node 6 → bucket 2
node 7 → bucket 3
INVALID nodes and workers correctly report is_valid() = false
```

## What to notice

- `PlacementHint::default()` is `Current` — the call-site NUMA node is the
  lowest-latency choice unless the caller has a specific reason to prefer
  another node or tier.

- The `describe_hint` function matches on `PlacementHint` exactly as a real
  allocator would.  The `Tier(_)` arm splits into two sub-patterns using a
  guard: `is_host_allocatable()` on the inner tier value, so the caller
  immediately knows whether a `Tier(Registers)` hint is a bug at the call site.

- `NumaBucketIndex::<4>` wraps node IDs modulo 4; `wrapping_add` stays inside
  the bucket count: `(2 + 3) % 4 = 1`. This allows a fixed-size placement
  table indexed by NUMA node without a bounds check in the hot path.

- `NumaNodeId::INVALID` and `WorkerId::INVALID` hold the maximum `u32` value
  and their `is_valid()` returns `false` — the sentinel is part of the type
  API, not a magic number that callers must remember.
