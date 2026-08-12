# 4. Topology Epoch

`TopologyEpoch` is a monotonically increasing counter that invalidates cached
topology snapshots when the hardware configuration changes.  It lives in
`src/law/epoch.rs`:

```rust,ignore
pub struct TopologyEpoch(u64);
impl TopologyEpoch {
    pub const INITIAL: Self;          // Self(0)
    pub const fn new(raw: u64) -> Self;
    pub const fn get(self) -> u64;
    pub const fn next(self) -> Self;  // wrapping_add(1)
}
```

## Semantics

`TopologyEpoch` is a pure counter.  It carries no wall-clock time, no
timestamp, and no causal ordering beyond "this epoch was observed before
that one."  Two epochs should be compared with `==` / `!=`, not `<` / `>`,
because the only meaningful question is whether the cached snapshot is still
current.

`INITIAL` (`Self(0)`) is the starting epoch for any freshly constructed
topology snapshot.  A live topology begins at `INITIAL` and advances each
time the hardware configuration changes.

`next()` is `wrapping_add(1)`.  Wrapping is safe in practice: at one hot-plug
event per second, a `u64` counter would take roughly 585 billion years to wrap.

## When the epoch advances

The epoch advances on any hot-plug event that changes the topology:

- A CPU or NUMA node is added to or removed from the system (e.g. ACPI-driven
  CPU hot-add, NUMA node online/offline).
- A GPU is inserted or removed (PCIe hot-plug or Thunderbolt).
- Any other event that invalidates the processor-to-node map, distance matrix,
  or device property table.

When the epoch advances, all [`CpuTopology`](cpu_topology.md) and
[`GpuTopology`](gpu_topology.md) snapshots captured under the previous epoch
are stale.

## Consumer pattern

The recommended caching pattern is:

```rust,ignore
struct TopologyCache {
    topology: CpuTopology,
    epoch: TopologyEpoch,
}

impl TopologyCache {
    fn refresh_if_stale(&mut self, live_epoch: TopologyEpoch) {
        if self.epoch != live_epoch {
            self.topology = CpuTopology::detect();
            self.epoch = live_epoch;
        }
    }
}
```

The component holds a snapshot together with the epoch at which it was
captured.  Before each use it compares its stored epoch against the current
live epoch.  If they differ it re-detects and updates both fields; if they are
equal the cached snapshot is valid and no work is done.

## Relationship to topology types

`TopologyEpoch` itself owns no topology data.  It is a validity token for
[`CpuTopology`](cpu_topology.md) and [`GpuTopology`](gpu_topology.md)
snapshots.  The live epoch is maintained by the platform integration layer
(outside themis scope); themis provides the type so that snapshots can carry
their epoch without a dependency on the platform layer.
