# 5. CPU Topology

`CpuTopology` is a snapshot of the host's NUMA layout, processor assignments,
inter-node distances, detected cache hierarchy, and reported processor
efficiency classes. It is obtained once (or re-detected after a
[`TopologyEpoch`](topology_epoch.md) change) and then shared read-only.

## Key types

### `NumaNode`

```rust,ignore
pub struct NumaNode {
    pub id: NumaNodeId,
    pub processors: Box<[u32]>,
    pub distances: Box<[u32]>,
    pub memory_tier: MemoryTier,
}
```

`processors` is the list of logical processor numbers that belong to this node.
`distances` is the NUMA distance vector for this node relative to all other
nodes in the topology. Linux reads
`/sys/devices/system/node/nodeN/distance`; Windows has no equivalent distance
source in the current backend and uses Themis's documented local/remote
fallback.
`memory_tier` classifies the memory attached to this node: `Dram` for ordinary
DDR DIMMs, `Hbm` for HBM-attached nodes, `Persistent` for NVDIMM nodes, etc.

### `CacheLevel`

```rust,ignore
pub struct CacheLevel {
    pub level: u32,
    pub size_bytes: usize,
    pub line_bytes: Option<usize>,
    pub shared_processors: Box<[u32]>,
}
```

`level` is 1, 2, or 3 for L1/L2/L3.  `line_bytes` is `None` when the OS did
not report a cache-line size (uncommon but possible in virtual environments).
`shared_processors` lists every logical processor that shares this cache
instance — relevant for work-stealing domain construction in moirai.

### `CpuTopology`

```rust,ignore
impl CpuTopology {
    #[cfg(feature = "std")]
    pub fn detect() -> Self;
    pub fn numa_node_for_processor(&self, processor: u32) -> Option<&NumaNode>;
}
```

`detect()` is only available when the `std` feature is enabled; it reads the
OS topology interfaces (`/sys/devices/system/cpu` on Linux, the NUMA API on
Windows).  The method returns a snapshot that is valid until the next
[`TopologyEpoch`](topology_epoch.md) advance.

`numa_node_for_processor(p)` is an O(1) lookup using an internally maintained
processor-to-node table.  It returns `None` only if `p` is greater than the
highest processor number present in the snapshot.

## Efficiency and native affinity

`CpuTopology::efficiency()` returns `None` when the platform did not report a
complete efficiency-class table. A returned `CpuEfficiencyView` proves that
presence once: class count, hybrid status, highest class, and class iteration
are then total. Only lookup by a caller-supplied processor id remains optional,
because that id can lie outside the snapshot.

Windows logical processor ids use Themis's flattened `group * 64 + bit`
numbering. On Windows, `ProcessorAffinityGroups::from_processors` owns the
inverse mapping to sorted native `(group, mask)` partitions. It deduplicates
repeated ids and retains ids that cannot fit the target's native mask in
`unassigned_processors()`. Consumers selecting one group can use
`largest_group()`; ties choose the lower group id deterministically. Consumers
capable of group-aware binding must use every returned partition rather than
silently collapsing them to one flat mask.

## Distance matrix

`NumaNode::distances` encodes the inter-node access latency in the same
normalised units the OS uses (10 = local, 20 = one hop, etc.).  Mnemosyne's
remote-access fallback uses this matrix to rank alternative nodes when the
preferred node is full: it picks the nearest node whose free capacity is
sufficient, rather than falling back to an arbitrary node.

## Relationship to placement types

[`NumaNodeId`](locality_identities.md) values in `PlacementHint::Numa(id)` are
resolved against `CpuTopology`.  Moirai uses `numa_node_for_processor` to
build work-stealing domains aligned with cache topology: threads that share an
L3 cache are placed in the same domain to maximise cache reuse before stealing
crosses the boundary.
