# 6. GPU Topology

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - GpuTopology: provider-fed (hephaestus supplies GpuDeviceProperties from
    wgpu adapter limits or CUDA device attributes); themis is stateless law
  - GpuDeviceProperties: all Option<NonZero*> fields — unknowability is
    type-level; sentinel zeros are unrepresentable
  - memory_tier: Hbm / Gddr / Device; drives mnemosyne's device-memory
    allocation policy
  - max_resident_warps(): derived capacity; None when any of the three
    required properties is unknown
  - Occupancy planning: kernel_budget = registers_per_unit / registers_per_thread;
    actual resident warps = min(kernel_budget, max_threads_per_unit/warp_width)
-->
