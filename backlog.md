# Backlog — themis

Strategic roadmap; tags `[patch]`/`[minor]`/`[major]`/`[arch]` per SemVer class.
themis is the Atlas placement-law SSOT: typed, stateless vocabulary that
mnemosyne (allocation), moirai (scheduling), and hephaestus (devices) consume.

## Delivered

- [x] [patch][arch] Keep `src/branded/region/mod.rs` as a thin module manifest.
  The `SyncRegionPlacement` implementation, NUMA-node proof helper, scope
  constructor, and unit tests now live in `region/scope.rs`; public exports and
  safety invariants are unchanged. ADR 0003 records the boundary. The locked
  all-target check, warning-denied Clippy, nextest `25/25`, doctests `5/5`,
  Rustdoc, and the Atlas conformance scan pass; the scan removes one
  `manifest_implementation` site without increasing another class.

- [x] [major] Close the placement-tag aliasing hole in `SyncRegionPlacement`.
  `split_static` duplicated the brand's single Melinoe write token and leaned
  on the NUMA node tag to keep the copies apart, but the tag was attached to a
  cell by the caller, so labelling one cell twice produced two live `&mut T`
  from safe code. Cell tags now come from ownership, an exclusive borrow
  (`from_unique`), or inheritance (`as_pinned_ref`); the four pinned traits are
  `unsafe` so downstream types discharge the same obligation. Decision:
  [ADR 0002](docs/adr/0002-placement-tags-are-proof-carrying.md). Evidence:
  reproduced pre-fix as a passing test and as a Miri Stacked Borrows error;
  post-fix `compile_fail` doctests pinning E0599 and E0499; 45/45 nextest, 5/5
  doctests, Miri clean over `src/branded/`, warning-denied Clippy on the
  pedantic floor across both feature sets and both platform backends.

- [x] [patch] Add verification CI (`ci.yml`): fmt, Clippy `-D warnings` on a
  `clippy::pedantic` + `unwrap_used` floor across the default and
  `--no-default-features` configurations, nextest, doctests, `cargo doc`, and
  Miri over `src/branded/`. Runs on a Linux/Windows matrix because the topology
  backends are disjoint per platform. All actions SHA-pinned; every job carries
  `timeout-minutes` and `permissions: contents: read`.

- [x] [patch] Publish future releases through a pinned GitHub Actions workflow
  using crates.io OIDC Trusted Publishing and no stored registry credential.

- [x] [major] Publish the placement-law crate as `themis-topology` because the
  `themis` crates.io namespace is owned by an unrelated project. Preserve
  `themis` as the library crate name so Rust imports remain stable. Decision:
  [ADR 0001](docs/adr/0001-crates-io-package-identity.md).

- [x] [patch] Resolve optional Melinoe from its default source, removing the
  revision identity that duplicated the Atlas provider graph. Evidence:
  formatting, all-feature check and warning-denied Clippy, value-semantic
  nextest, doctest, rustdoc, and a single resolved Melinoe source.

- [x] [minor] Replace fabricated CPU cache defaults with provider-reported
  cache levels. Linux sysfs and Windows native cache relationships are parsed
  into `Option<Box<[CacheLevel]>>`; unavailable data stays absent. Leto and
  Moirai consumers preserve the typed absence. Evidence: provider nextest
  50/50 and clippy/doc gates; consumer contract tests land in the dependent
  increments.

- [x] [patch] Update the optional Melinoe dependency to 0.9.0 as part of the
  executor-safety co-evolution sweep. Evidence: Clippy, 50/50 nextest, doctests,
  and rustdoc pass under all features; Themis value semantics are unchanged.

- [x] [patch] Add default `parallel` and `mnemosyne-memory` feature markers to
  the placement-law crate. Evidence: `cargo metadata --no-deps --locked
  --format-version 1`; full Atlas feature-policy metadata audit; `cargo fmt
  --check`; `git diff --check`. Residual: compile/test gates were blocked before
  rustc by denied access to `target/debug/.cargo-lock`.

## Heterogeneous topology law (atlas ADR 0002) [arch]

Extend the law to the full CPU/GPU/TPU memory and compute hierarchy. themis
stays stateless: detection may be provider-fed (hephaestus reports device
properties into themis types).

- [x] [minor] (0.6.0) `MemoryTier` additions: `Gddr` (distinct from `Hbm`; both are
  device-attached but with different bandwidth/latency law) and `HostPinned`
  (page-locked host staging). Document the tier lattice and intended consumers.
- [x] [minor] (0.6.0) Budgeted device tiers: `Registers` and `SharedMem` as
  *non-host-allocatable* tiers with queryable capacities (regs per SM/thread,
  shared bytes per SM/block). These exist so mnemosyne's kernel resource
  budgets and moirai's occupancy planner speak themis types — host code never
  allocates them (GPU compilers assign registers; ADR 0002 constraint 2).
- [x] [minor] (0.6.0) `GpuTopology` snapshot alongside `CpuTopology`: SM/CU count,
  warp/wavefront width, max threads per SM, register file size per SM,
  shared memory per SM, L2 size, memory tier (Hbm/Gddr) + bandwidth class.
  Provider-fed constructor (hephaestus supplies values from wgpu adapter info
  / CUDA device attributes).
- [x] [minor] (0.9.0) Minimal `TpuTopology` vocabulary (core count, HBM
  capacity per core) — types only, no detection, gated on a real PJRT consumer
  in hephaestus. Evidence: provider-fed `TpuTopology`/`TpuDeviceProperties`,
  value-semantic tests including saturating total-capacity derivation, local
  verification gates recorded in the change commit. Residual: `cargo
  semver-checks check-release` is blocked by historical published
  `themis 0.0.3` resolving yanked `zeroize 0.5.2` before compatibility
  comparison.
- [x] [patch] `CacheLevel` consumers note: document the leto tiling and
  moirai chunking contracts that read L1/L2/L3 sizes, so changes to the type
  surface treat them as consumers. Evidence: README boundary/evidence sync;
  `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D
  warnings`; `cargo nextest run`; `cargo test --doc`; `cargo doc --no-deps`.

## Verification infrastructure [patch]

- [x] [patch] Commit the nextest timeout gate at `.config/nextest.toml`
  (`30s` slow threshold, hard stop after two periods). Evidence:
  `cargo nextest run`.
- [x] [patch] Add a stable, dependency-free topology benchmark target that
  exercises real `CpuTopology::single_node` construction and lazy
  processor-to-node traversal. Evidence: `cargo bench --no-run`; `cargo
  bench`. Benchmark tier: empirical harness output only, no statistical
  baseline or speedup claim.
- [x] [patch] Document the topology benchmark command and evidence tier in the
  README. Evidence: doc sync plus verification gates recorded in the change
  commit.

## Locality query performance [patch]

- [x] [patch] Consolidate current-processor and current-NUMA OS probes through
  one internal locality query path so cache refresh and processor reads share
  one platform syscall/API implementation. Evidence: clippy/test/doc gates;
  empirical benchmark tier from `cargo nextest run` wall-time only, no
  criterion baseline claimed.
- [x] [patch] Align Linux current-locality cfg gates with the `getcpu` probe so
  non-x86_64 Linux targets do not also compile the unsupported-platform
  fallback branch. Evidence: cfg audit plus verification gates recorded in the
  change commit. Residual: local Linux target check was blocked before themis
  by missing `core` for `x86_64-unknown-linux-gnu` in dependency compilation.
- [x] [patch] Split current-locality query code into cache, platform, and test
  leaf modules. Evidence: module-level ownership separation; no public API
  change; verification gates recorded in the change commit.

## CPU topology structure [patch]

- [x] [patch] Split CPU topology into snapshot/accessor, platform detection,
  dense-table builder, and cache-default leaf modules. Evidence: no public API
  change; verification gates recorded in the change commit.
- [x] [patch] Centralize default NUMA distance-row construction in the CPU
  table builder so platform detector fallbacks share one `10` local / `20`
  remote law. Evidence: value-semantic row test plus verification gates
  recorded in the change commit.
- [x] [patch] Name the default NUMA local and remote distance constants so
  topology code and tests share one internal SSOT for fallback distance values.
  Evidence: value-semantic tests plus verification gates recorded in the
  change commit.
- [x] [patch] Split CPU platform detection into Linux sysfs and Windows API
  leaf modules. Evidence: no public API change; value-semantic CPU topology
  tests retained; verification gates recorded in the change commit.
- [x] [patch] Build `CpuTopology::single_node` processor-to-node storage
  directly, removing the temporary processor/node pair allocation. Evidence:
  value-semantic CPU topology tests retained; verification gates recorded in
  the change commit. Benchmark tier: bench-profile compilation and test
  harness only, no criterion latency/allocation baseline claimed.
- [x] [patch] Build fixed-size single-node NUMA rows and default cache-level
  rows as boxed arrays instead of temporary vectors. Evidence: value-semantic
  CPU topology tests retained; verification gates recorded in the change
  commit. Benchmark tier: code-inspection allocation cleanup plus bench-profile
  compilation, no criterion latency/allocation baseline claimed.
- [x] [patch] Add value-semantic coverage for conservative CPU cache defaults,
  including L1/L2 private rows and L3 shared-processor membership. Evidence:
  `cargo nextest run` plus verification gates recorded in the change commit.
- [x] [patch] Mark `CpuTopology::processor_node_pairs()` as must-use so callers
  get compiler feedback when discarding the lazy processor-to-node traversal.
  Evidence: compiler-enforced API annotation; verification gates recorded in
  the change commit.
- [x] [patch] Pre-size Windows NUMA detector processor buffers from the
  reported logical processor count and per-node processor mask population.
  Evidence: code-inspection allocation cleanup plus verification gates recorded
  in the change commit; no latency/allocation benchmark baseline claimed.

## Topology type hierarchy [patch]

- [x] [patch] Split topology structural types into CPU, GPU, and TPU leaf
  modules. Evidence: no removal of existing public exports; verification gates
  recorded in the change commit.

## Topology test hierarchy [patch]

- [x] [patch] Split topology tests into CPU, GPU, and TPU leaf modules.
  Evidence: value-semantic assertions preserved; verification gates recorded
  in the change commit.
- [x] [patch] Replace topology smoke-test assertions with value-semantic
  checks for detected NUMA lookup behavior and memory-tier host-allocation
  contracts. Evidence: `cargo nextest run` plus verification gates recorded in
  the change commit.

## Placement law structure [patch]

- [x] [patch] Split placement law value types into identity, epoch, memory, and
  placement leaf modules. Evidence: no public API removal; value-semantic tests
  retained in the law module; verification gates recorded in the change commit.
- [x] [patch] Remove redundant `NumaNodeId::bucket_index` pre-normalization so
  zero bucket counts use the documented domain panic and valid counts perform
  one modulo operation. Evidence: regression panic-contract test plus
  verification gates recorded in the change commit.
- [x] [patch] Centralize the const-generic nonzero bucket invariant behind one
  private helper used by `NumaBucketIndex` construction and wrapping
  arithmetic. Evidence: existing panic-contract regression plus verification
  gates recorded in the change commit.

## Branded placement structure [patch]

- [x] [patch] Split Melinoe-backed branded placement scopes into
  thread-confined, sync-region, and test leaf modules. Evidence: no
  feature-gated public API removal; value-semantic branded-scope tests
  retained; verification gates recorded in the change commit.

## Melinoe branded-collection adoption [minor]

- [x] [minor] (0.10.1) Adopt Melinoe collections for
  `NumaPinnedSlice`/`ConstNumaPinnedSlice` construction and `from_fn`
  generation, and `partition_for_each_mut_with` on dynamic and const
  placement permits — preserving Themis-owned placement identity while
  eliminating duplicated `Vec<T>` → `Vec<MelinoeCell<T>>` construction.
  Also lands the no-`std` `alloc` fix, removal of the dead no-`std`
  `detect_cache_levels` fallback, `melinoe/alloc` feature wiring, README
  boundary note, and two new branded tests. Cross-link:
  ATLAS-THEMIS-MELINOE-ADOPTION-002 (`cad222b`); evidence: strict Clippy,
  Nextest 21/21 default + 38/38 `testing` + 21/21 `--no-default-features`.

## Residual findings (this cycle) [patch]

- [x] [patch] `compile_fail` error codes were enforced only by the nightly
  doctest job: stable rustdoc parses `compile_fail,E0499` and never checks the
  code (verified by feeding it a deliberately wrong code — stable passed,
  nightly reported "Some expected error codes were not found"). Stable
  `trybuild` UI tests now pin the `E0599` shared-cell construction failure and
  the `E0499` overlapping-borrow failure in committed `.stderr` fixtures.
  `cargo nextest run --features "melinoe testing" --lib --test branded
  --test compile_fail` passes 42/42; the nightly doctest remains the independent
  exact-code check.

- [x] [patch] The repository now carries `.gitattributes` with the canonical
  `* text=auto` normalization, so source blobs do not depend on each
  contributor's `core.autocrlf`. The tracked file is present at the exact
  default head `fa8dc29`; no tree-wide renormalization was needed because the
  existing index is already normalized.

## ADR governance — generated index refresh

- **[patch]** Refresh `docs/adr/README.md` from the existing canonical
  `Accepted` headers for ADR 0001–0002. No decision content or provider
  contract changes are in scope; Atlas root `ATLAS-ADR-GOV-058` owns the
  cross-repository burn-down.
