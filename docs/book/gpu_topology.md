# 6. GPU Topology

`GpuTopology` holds the device properties reported by the GPU backend.
Themis owns the types; hephaestus populates them from wgpu adapter limits or
CUDA device attributes and hands the snapshot to moirai's occupancy planner.
Themis itself never queries a driver.

## `GpuDeviceProperties`

```rust,ignore
pub struct GpuDeviceProperties {
    pub compute_units:           Option<NonZeroU32>,
    pub warp_width:              Option<NonZeroU32>,
    pub max_threads_per_unit:    Option<NonZeroU32>,
    pub registers_per_unit:      Option<NonZeroU32>,
    pub shared_mem_per_unit_bytes: Option<NonZeroUsize>,
    pub l2_bytes:                Option<NonZeroUsize>,
    pub memory_tier:             MemoryTier,
    pub memory_bytes:            Option<NonZeroU64>,
}
```

Every capacity field is `Option<NonZero*>`.  A field the driver did not report
is `None`; a field the driver reported as zero cannot be represented.  This
design makes unknowability explicit at the type level: there is no sentinel
zero that could silently produce a divide-by-zero in the occupancy formula.

**`compute_units`** is the number of streaming multiprocessors (NVIDIA), compute
units (AMD), or equivalent parallel execution units.

**`warp_width`** is the SIMD width of one warp or wavefront — 32 for NVIDIA,
64 for most AMD GCN/RDNA.

**`max_threads_per_unit`** is the maximum number of resident threads per
compute unit as reported by the driver.

**`registers_per_unit`** is the total register file size per compute unit,
in 32-bit register units.

**`shared_mem_per_unit_bytes`** is the shared-memory capacity per compute unit
available for kernel use.

**`memory_tier`** classifies the device memory: `Hbm` for A100-class devices,
`Gddr` for consumer GPUs.  When `is_host_allocatable()` is `true` on this
tier, mnemosyne may create device-memory pools accessible from the host.

**`memory_bytes`** is total device-local memory capacity.

## Occupancy planning

The resident-warp formula is:

```rust,ignore
// max_resident_warps = compute_units * max_threads_per_unit / warp_width
// (returns None if any input is None)
```

Register-limited occupancy for a kernel using `registers_per_thread` registers:

```text
kernel_budget  = registers_per_unit / (registers_per_thread * warp_width)
resident_warps = min(kernel_budget, max_threads_per_unit / warp_width)
```

Shared-memory-limited occupancy for a kernel using `shared_per_block` bytes:

```text
blocks_per_unit = shared_mem_per_unit_bytes / shared_per_block
```

Because all fields are `Option<NonZero*>`, the planner can detect partial
information and fall back to a conservative block size rather than crashing.
Note that wgpu's abstract API does not expose `compute_units`, so
`max_resident_warps()` will be `None` for wgpu-provided properties.

## Provider responsibility

Hephaestus reads the relevant adapter or device properties and constructs a
`GpuDeviceProperties`.  Themis provides no `detect()` method; it is a
pure-data crate with no driver dependency.  The snapshot is valid until the
next [`TopologyEpoch`](topology_epoch.md) advance (e.g. GPU hot-unplug).

## Relationship to `MemoryTier`

`GpuDeviceProperties::memory_tier` connects to the same
[`MemoryTier`](memory_tiers.md) enum used in `PlacementHint::Tier`.  A
caller that targets HBM device memory writes
`PlacementHint::Tier(MemoryTier::Hbm)`; the allocator resolves this against
`GpuTopology` to find the matching device pool.

See the [worked example](examples/gpu_topology.md) for a full A100-like
occupancy calculation.
