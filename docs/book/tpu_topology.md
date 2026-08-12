# 7. TPU Topology

`TpuTopology` and `TpuDeviceProperties` carry the device properties of
matrix-multiplication accelerators: Google TPUs, Apple Neural Engine, Intel
Gaudi, and similar systolic-array devices.  Themis owns the types on the same
terms as [`GpuTopology`](gpu_topology.md): it defines the vocabulary; the
backend (hephaestus or a platform-specific adapter) populates the snapshot.

## `TpuDeviceProperties`

```rust,ignore
pub struct TpuDeviceProperties {
    pub core_count:          u32,
    pub hbm_bytes_per_core:  u64,
}
```

Unlike `GpuDeviceProperties`, these fields are plain integers rather than
`Option<NonZero*>`.  A value of `0` means unreported: the device is present
but the backend did not obtain that attribute.  Callers should treat a zero
`core_count` as "unknown" and avoid dividing by it.

**`core_count`** is the number of tensor-processing cores available to the
runtime.  For a Google TPU v4 pod slice, this is the number of chips multiplied
by the chips-per-slice count visible to the process.

**`hbm_bytes_per_core`** is the HBM capacity per core.  Total device-local HBM
is `core_count * hbm_bytes_per_core` when `core_count > 0`.

## Usage in moirai

Moirai's task router uses `TpuTopology` to decide whether to dispatch a tensor
operation to a TPU device.  The routing logic is:

1. Check that `core_count > 0` (device is present and reported).
2. Verify that the operation's working-set fits within
   `hbm_bytes_per_core * core_count`.
3. If both conditions hold, route to the TPU; otherwise fall back to GPU or
   CPU execution.

Themis does not implement this routing logic; it provides the types that
moirai's router reads.

## Relationship to `MemoryTier`

TPU HBM maps to `MemoryTier::Hbm`.  A `PlacementHint::Tier(MemoryTier::Hbm)`
hint may resolve to a TPU device pool as well as a GPU device pool, depending
on which devices the platform reports.  The allocator consults the full device
topology (both `GpuTopology` and `TpuTopology`) when resolving tier-based
hints.

## Topology epoch

Like all topology snapshots, `TpuDeviceProperties` is valid until the next
[`TopologyEpoch`](topology_epoch.md) advance.  A TPU device being detached or
re-enumerated increments the epoch and invalidates cached snapshots.
