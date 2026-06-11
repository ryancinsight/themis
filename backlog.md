# Backlog — themis

Strategic roadmap; tags `[patch]`/`[minor]`/`[major]`/`[arch]` per SemVer class.
themis is the Atlas placement-law SSOT: typed, stateless vocabulary that
mnemosyne (allocation), moirai (scheduling), and hephaestus (devices) consume.

## Heterogeneous topology law (atlas ADR 0002) [arch]

Extend the law to the full CPU/GPU/TPU memory and compute hierarchy. themis
stays stateless: detection may be provider-fed (hephaestus reports device
properties into themis types).

- [ ] [minor] `MemoryTier` additions: `Gddr` (distinct from `Hbm`; both are
  device-attached but with different bandwidth/latency law) and `HostPinned`
  (page-locked host staging). Document the tier lattice and intended consumers.
- [ ] [minor] Budgeted device tiers: `Registers` and `SharedMem` as
  *non-host-allocatable* tiers with queryable capacities (regs per SM/thread,
  shared bytes per SM/block). These exist so mnemosyne's kernel resource
  budgets and moirai's occupancy planner speak themis types — host code never
  allocates them (GPU compilers assign registers; ADR 0002 constraint 2).
- [ ] [minor] `GpuTopology` snapshot alongside `CpuTopology`: SM/CU count,
  warp/wavefront width, max threads per SM, register file size per SM,
  shared memory per SM, L2 size, memory tier (Hbm/Gddr) + bandwidth class.
  Provider-fed constructor (hephaestus supplies values from wgpu adapter info
  / CUDA device attributes).
- [ ] [minor] Minimal `TpuTopology` vocabulary (core count, HBM capacity per
  core) — types only, no detection, gated on a real PJRT consumer in
  hephaestus.
- [ ] [patch] `CacheLevel` consumers note: document the leto tiling and
  moirai chunking contracts that read L1/L2/L3 sizes, so changes to the type
  surface treat them as consumers.
