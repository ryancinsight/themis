# Changelog

## 0.2.0 - 2026-06-09

### Changed

- Replaced `CpuTopology::processor_to_node` tree storage with a dense indexed table for O(1) processor lookup and lower per-entry allocation overhead on dense CPU IDs.
- Added `CpuTopology::node_to_index`, `CpuTopology::node_index`, and `CpuTopology::processor_node_pairs` as the canonical topology accessors for consumers.

### Breaking

- `CpuTopology::processor_to_node` is now `Vec<Option<NumaNodeId>>` instead of `BTreeMap<u32, NumaNodeId>`.

### Migration

- Replace direct map iteration with `CpuTopology::processor_node_pairs()`.
