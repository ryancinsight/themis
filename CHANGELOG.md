# Changelog

## Unreleased

### Fixed

- **Cache levels exclude instruction and trace caches.** Both providers read a
  cache's type and report only data-holding caches (Windows
  `PROCESSOR_CACHE_TYPE` Unified/Data; Linux sysfs `type` Data/Unified). A
  split-L1 core exposes its instruction and data caches with the *same*
  `CacheLevel::level`, and the type was the only field distinguishing them, so a
  consumer reducing levels by `level` silently resolved L1 to whichever entry it
  scanned last. On a hybrid Arrow Lake host this reported 48 KiB (P-core L1d),
  32 KiB (E-core L1d) and 64 KiB (L1i) all as level 1, and a downstream
  consumer's published L1 figure — meant for data blocking — resolved to the
  64 KiB instruction cache. Detection on that host now returns 37 levels instead
  of 61, with no instruction cache among them. Linux treats absent or unreadable
  `type` as data-holding, so a kernel that does not expose it keeps every level
  rather than losing all of them.

### Added

- **Core efficiency class.** `CpuTopology` now models the performance/efficiency
  distinction on hybrid CPUs. `EfficiencyClass` is a dense ordinal — higher is
  more performant, three-tier parts are representable — reached through
  `efficiency_classes`, `processor_efficiency_class`, `efficiency_class_count`,
  `is_hybrid`, `highest_efficiency_class`, and
  `processors_in_efficiency_class`. Windows reads the `EfficiencyClass` byte of
  each `GetLogicalProcessorInformationEx(RelationProcessorCore)` record; Linux
  reads the Intel hybrid CPU-type `cpulist`s and then ARM `cpu_capacity`; every
  other target reports absence, as does any host whose data does not cover every
  logical processor. A host is never inferred to be hybrid from core counts,
  processor ids, or model strings. `efficiency_class_count` is the absence
  oracle: `None` is "not reported", `Some(1)` is a homogeneous host. This
  retires the machine-specific performance-core constants hand-rolled in apollo's
  pinned probes and mnemosyne's benchmark harness, both of which mislabelled the
  developer host. See
  [ADR 0004](docs/adr/0004-efficiency-class-is-an-ordinal.md).

### Fixed

- **Soundness.** `SyncRegionPlacement::split_static` duplicated the brand's
  single Melinoe write token and relied on the NUMA node tag to keep the copies
  apart, but the tag was attached to a cell by the caller. Labelling one cell
  with two node ids produced two live `&mut T` to the same location from
  entirely safe code (confirmed under Miri as a Stacked Borrows violation). A
  cell's tag now comes from a construction path that proves it. See
  [ADR 0002](docs/adr/0002-placement-tags-are-proof-carrying.md).

### Breaking

- The four `from_unique` constructors are no longer `const fn`: Rust 1.81
  rejects `&mut` parameters in `const fn`. Their runtime behavior is unchanged.
- Removed `NumaPinnedCellRef::new`, `NumaPinnedSliceRef::new`,
  `ConstNumaPinnedCellRef::new`, and `ConstNumaPinnedSliceRef::new`. These
  accepted a shared cell reference plus a caller-chosen node tag, which is the
  aliasing hole. Replace with `from_unique`, which takes `&mut` — the exclusive
  borrow is the placement proof — or with `as_pinned_ref` on the owning pinned
  type, which inherits the owner's tag.
- `PinnedCell`, `ConstPinnedCell`, `PinnedSlice`, and `ConstPinnedSlice` are now
  `unsafe` traits. They are the dispatch surface the placement `write` methods
  use, so implementors must guarantee their cell is unreachable under any other
  node tag. Existing implementors add `unsafe` to the `impl`.

### Changed

- Declare Rust 1.81 as the library MSRV and verify it on Linux and Windows.
- `SyncRegionPlacement::project_static` is no longer `unsafe`. It consumes the
  region and returns a single capability, so it never duplicates the token and
  imposes no obligation on the caller.
- `split` and `split_with` assert pairwise-distinct NUMA node ids, discharging
  locally the precondition their `unsafe` token duplication depends on.
- Added a `ci.yml` workflow: fmt, clippy under `-D warnings` across both feature
  configurations, nextest, doctests, `cargo doc`, a nightly doctest pass that
  enforces `compile_fail` error codes (stable rustdoc ignores them), and Miri
  over `src/branded/`.
- Added a `[lints]` floor: `clippy::pedantic` plus `clippy::unwrap_used`.
- Add a GitHub Release workflow that validates crate identity and package
  contents before publishing through crates.io Trusted Publishing.
- Publish under the collision-free `themis-topology` package name while
  retaining `themis` as the Rust library crate name.
- Resolve the optional Melinoe dependency from its default source, removing the
  revision identity that duplicated the provider graph in Atlas consumers.

## 0.10.0 - 2026-07-14

### Changed

- Updated the optional Melinoe contract to 0.9.0 so placement-law consumers
  share the validated parallel-executor capability version. Themis does not use
  the changed registration API, so its public surface is unchanged.
- Replaced fabricated CPU cache defaults with provider-reported cache levels.
  Linux reads sysfs cache indices and Windows reads the native logical
  processor cache relationship; unavailable or malformed data is typed absence.
  `CpuTopology::cache_levels` now returns `Option`, and each reported level
  carries an optional provider line size.

### Breaking

- `CpuTopology::cache_levels()` returns `Option<&[CacheLevel]>`; consumers must
  handle unavailable cache topology instead of reading synthetic defaults.

### Migration

- Match on `cache_levels()` before deriving cache-aware tiling or locality
  policy. `CacheLevel::line_bytes` is optional because providers may omit line
  size even when they report cache capacity.

## 0.9.17 - 2026-06-17

### Changed

- Added value-semantic coverage for conservative CPU cache defaults, including
  L1/L2 private rows and L3 shared-processor membership.

## 0.9.16 - 2026-06-17

### Fixed

- Aligned Linux current-locality cfg gates with the `getcpu` probe so non-x86_64
  Linux targets do not also compile the unsupported-platform fallback branch.

## 0.9.15 - 2026-06-15

### Changed

- Centralized the const-generic nonzero bucket invariant behind one private
  helper used by `NumaBucketIndex` construction and wrapping arithmetic.

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
