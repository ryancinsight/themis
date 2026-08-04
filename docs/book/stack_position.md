# 8. Position in the Stack

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - Atlas layer: eunomia → aequitas → themis (parallel to mnemosyne / moirai)
  - themis feeds mnemosyne: PlacementHint selects the allocator's NUMA policy;
    MemoryTier selects the memory backend (DRAM, HBM, pinned)
  - themis feeds moirai: WorkerId drives worker affinity; NumaNodeId drives
    task routing; CpuTopology drives work-stealing domain boundaries
  - hephaestus populates GpuTopology: hephaestus reads wgpu adapter limits or
    CUDA device attributes, wraps them in GpuDeviceProperties, and hands the
    snapshot to moirai's occupancy planner
  - What themis does NOT own: actual allocation (mnemosyne), actual scheduling
    (moirai), branded capability evidence (melinoe / melinoe::halo)
-->
