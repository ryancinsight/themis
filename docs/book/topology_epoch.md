# 4. Topology Epoch

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - TopologyEpoch: monotonically increasing counter; INITIAL is the starting
    epoch for any freshly-constructed topology snapshot
  - When epoch advances: hot-plug events (CPU/NUMA node add/remove, GPU
    insertion) invalidate cached CpuTopology/GpuTopology snapshots
  - Consumer pattern: cache the topology snapshot alongside its epoch;
    re-query when the epoch in the live topology differs from the cached one
  - No clock, no time: epoch is a pure counter, not a timestamp — comparison
    is always == / !=, never < or > for freshness
-->
