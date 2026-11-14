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

## Binding the calling thread

`bind_current_thread(processor)` confines the calling thread to exactly one
logical processor, using the same flattened numbering.
`bind_current_thread_to(&ThreadAffinity)` confines it to a set and returns an
`AffinityChange`: `previous()` is the affinity replaced and `effective()` the
affinity read back, which can be a proper subset of the request because Linux
intersects it with the online, permitted processors. `current_thread_affinity()`
reports the present affinity. Windows calls `SetThreadGroupAffinity`, narrowing
the request to the active processors of its group first, so one call binds the
processors of one group; Linux calls `sched_setaffinity` with a 4 KiB bitmap
covering the crate's 32 768-processor bound. None of these allocate or panic,
and none needs the `std` feature.

There is no guard. A caller restores with
`bind_current_thread_to(change.previous())`, which replaces whatever was set in
between and returns `Err` when the kernel refuses. On a Windows 11 host with
more than 64 processors, a thread never bound spans every group but reports its
primary group only, so restoring it narrows to that group.

A call succeeds only when the operating system accepted the request, so a
scheduler publishes a worker-to-processor assignment after `Ok` and refuses to
start the worker on `Err`. `BindError::Unsupported` reports a target with no
backend, `OutOfRange { processor }` an id the target cannot name,
`SpansGroups { groups }` a Windows set in several groups, `NoActiveProcessor` an
empty set or a Windows group or processor the host lacks, and `Os { code }` a
kernel refusal (for example `EINVAL` for a processor outside the process's
cpuset). A failed call leaves the thread's affinity unchanged. Rationale:
ADR 0006.

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
