# 2. Memory Tiers

*Chapter prose deferred — DoR item.*

<!-- Key points to cover:
  - MemoryTier enum: Dram, Hbm, Gddr, HostPinned, Device, Persistent,
    Registers, SharedMem
  - is_host_allocatable: Registers and SharedMem are budget vocabulary only,
    not addressable by the host allocator
  - Registers and SharedMem: GPU compiler assigns registers; kernel launch
    declares shared memory; mnemosyne reads these as occupancy budgets
  - Hbm vs. Gddr: both are device memory; the distinction matters for
    bandwidth-locality modelling (A100 HBM2e vs. RTX 4090 GDDR6X)
  - Tier(MemoryTier) inside PlacementHint: callers can request HBM without
    knowing which NUMA node holds it
-->
