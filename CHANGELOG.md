# Changelog

## Unreleased

## 0.9.14 - 2026-06-15

### Fixed

- Removed redundant pre-normalization in `NumaNodeId::bucket_index` so zero
  bucket counts use the documented domain panic and valid bucket counts perform
  one modulo operation.

## 0.9.13 - 2026-06-15

### Changed

- Named the default NUMA local and remote distance constants so topology code
  and tests share one internal source of truth for fallback distance values.

## 0.9.12 - 2026-06-15

### Fixed

- Centralized default NUMA distance-row construction so detector fallbacks use
  one `10` local / `20` remote distance law instead of duplicating the rule.

## 0.9.11 - 2026-06-14

### Changed

- Replaced topology smoke-test assertions with value-semantic checks for
  queryable NUMA node indices, self-distance, adjacency length, and explicit
  memory-tier host-allocation contracts.

## 0.9.10 - 2026-06-14

### Changed

- Documented the topology benchmark command and its evidence tier in the
  README so benchmark output is not mistaken for a statistical baseline.

## 0.9.9 - 2026-06-13

### Added

- Added a dependency-free topology benchmark target that measures real
  single-node topology construction and processor-to-node iterator traversal
  under `cargo bench`.

## 0.9.8 - 2026-06-13

### Changed

- Pre-sized Windows NUMA processor mapping buffers from the logical processor
  count and per-node processor masks, avoiding growth reallocations while
  preserving detected topology values.

## 0.9.7 - 2026-06-13

### Changed

- Marked `CpuTopology::processor_node_pairs()` as must-use so ignored lazy
  mapping traversals produce compiler feedback without changing topology
  storage or iteration semantics.

## 0.9.6 - 2026-06-12

### Changed

- Split CPU platform topology detection into `cpu/detect/{linux,windows}.rs`,
  keeping OS-specific sysfs and Windows API logic in separate leaf modules
  behind the existing `CpuTopology::detect()` entry point.

## 0.9.5 - 2026-06-12

### Changed

- Built fixed-size single-node NUMA rows and default cache-level rows as boxed
  arrays, removing temporary vector construction for fixed topology tables
  while preserving the same public topology values.

## 0.9.4 - 2026-06-12

### Changed

- Built `CpuTopology::single_node` processor-to-node storage directly as a
  dense boxed slice, removing the temporary processor/node pair allocation
  while preserving the same single-node topology semantics.

## 0.9.3 - 2026-06-12

### Changed

- Split branded placement scopes into
  `branded/{thread_local,sync_region,tests}.rs`, keeping thread-confined and
  thread-portable Melinoe capabilities in separate leaf modules without
  changing the feature-gated public API.

## 0.9.2 - 2026-06-12

### Changed

- Split topology tests into `topology/tests/{cpu,gpu,tpu}.rs`, keeping each
  topology family’s value-semantic assertions with its bounded context.

## 0.9.1 - 2026-06-12

### Changed

- Split placement law value types into `law/{identity,epoch,memory,placement}.rs`
  so ID newtypes, topology epoch, memory tiers, and placement hints have
  separate module ownership without changing the public API.

## 0.9.0 - 2026-06-12

### Added

- `TpuTopology` and `TpuDeviceProperties` provider-fed vocabulary for TPU core
  count and HBM capacity per core, plus saturating total-HBM capacity
  derivation.
- Default `parallel` and `mnemosyne-memory` feature markers for the placement
  law crate, keeping Atlas provider feature policy uniform without adding a
  runtime dependency or changing topology semantics.
- Committed `.config/nextest.toml` with the 30s slow-test and 60s termination
  gate used by the local verification policy.

### Changed

- Consolidated current CPU and NUMA OS probing through one internal locality
  query implementation, removing duplicate platform syscall/API code without
  changing the public query contract.
- Split current-locality queries into `query/{cache,platform,tests}.rs` so TLS
  caching, OS probing, and verification have separate module ownership without
  changing the public query API.
- Split CPU topology into `topology/cpu/{mod,detect,tables,cache}.rs`, keeping
  snapshot accessors, OS detection, dense lookup construction, and conservative
  cache defaults in separate leaf modules without changing the public topology
  API.
- Split topology structural types into `topology/types/{cpu,gpu,tpu}.rs`, so
  CPU, GPU, and TPU provider vocabulary have separate module ownership.
- Documented `CacheLevel` consumers: leto uses cache sizes for tiling hints,
  and moirai uses shared-processor rows for chunk-locality hints.

## 0.8.0 - 2026-06-12

### Changed

- Thread-local NUMA cache now uses the melinoe `thread_cached!` SSOT
  (0.7.0), deleting the crate-local nightly/stable TLS pair. melinoe is now
  an unconditional dependency (it is no_std, default-features off); the
  `melinoe` cargo feature continues to gate only the branded placement
  extras.

## 0.7.0 - 2026-06-12

### Added

- `try_current_numa_node() -> Option<NumaNodeId>`: uncached NUMA-node query
  preserving the stack-wide "unreported = None, never fabricated" contract for
  consumers that must distinguish node 0 from unknown (driver: hermes NUMA
  detection consolidation onto themis). `current_numa_node` keeps its cached
  node-0 fallback for placement decisions.

## 0.6.0 - 2026-06-11

### Added

- atlas ADR 0002 tier/topology vocabulary: `MemoryTier::{Gddr, HostPinned,
  Registers, SharedMem}`. `Registers`/`SharedMem` are budgeted, non-host-
  allocatable device tiers (GPU compilers assign registers; kernels declare
  shared memory) encoded by the new `MemoryTier::is_host_allocatable`.
- `GpuTopology` snapshot + `GpuDeviceProperties` provider struct: SM/CU count,
  warp width, max threads per unit, registers per unit, shared memory per
  unit, L2 size, global-memory tier and capacity, with a
  `max_resident_warps` occupancy helper. Provider-fed (hephaestus reports wgpu
  adapter limits / CUDA device attributes); themis stays stateless law.
  Unreported fields are zero, never fabricated.

## 0.5.0 - 2026-06-09

### Added

- Added `NumaBucketIndex<const BUCKETS: usize>` and `NumaNodeId::bucket_index` as the canonical fixed-table NUMA placement normalization API.

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
