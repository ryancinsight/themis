# Changelog

## 0.4.0 - 2026-06-09

### Changed

- Precomputed adjacent NUMA node order in `CpuTopology` so locality consumers can read steal order without per-call allocation or sorting.

### Breaking

- `CpuTopology::adjacent_nodes` now returns `&[NumaNodeId]` instead of allocating `Vec<NumaNodeId>`.

### Migration

- Iterate the returned slice directly or call `.to_vec()` at the outer boundary when owned storage is required.

## 0.3.0 - 2026-06-09

### Changed

- Moved `CpuTopology` storage behind accessor methods and boxed slice tables to reduce snapshot heap metadata and decouple consumers from dense-index representation.
- Converted `NumaNode` processor and distance rows plus cache shared-processor rows to boxed slices.

### Breaking

- `CpuTopology` fields are no longer public. Use `epoch()`, `numa_nodes()`, `cache_levels()`, `logical_processors()`, `processor_to_numa_node()`, `processor_node_pairs()`, `node_index()`, `distance()`, and `adjacent_nodes()`.
- `NumaNode::processors`, `NumaNode::distances`, and `CacheLevel::shared_processors` are `Box<[u32]>` instead of `Vec<u32>`.

### Migration

- Replace direct `CpuTopology` field access with accessor methods.

## 0.2.0 - 2026-06-09

### Changed

- Replaced `CpuTopology::processor_to_node` tree storage with a dense indexed table for O(1) processor lookup and lower per-entry allocation overhead on dense CPU IDs.
- Added `CpuTopology::node_to_index`, `CpuTopology::node_index`, and `CpuTopology::processor_node_pairs` as the canonical topology accessors for consumers.

### Breaking

- `CpuTopology::processor_to_node` is now `Vec<Option<NumaNodeId>>` instead of `BTreeMap<u32, NumaNodeId>`.

### Migration

- Replace direct map iteration with `CpuTopology::processor_node_pairs()`.
