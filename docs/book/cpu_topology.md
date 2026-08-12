# 5. CPU Topology

`CpuTopology` is a snapshot of the host's NUMA layout, processor assignments,
inter-node distances, and detected cache hierarchy.  It is obtained once (or
re-detected after a [`TopologyEpoch`](topology_epoch.md) change) and then
shared read-only.

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
nodes in the topology — the same values exposed by the OS (Linux
`/sys/devices/system/node/nodeN/distance`, Windows `GetNumaNodeProcessorMask`).
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
