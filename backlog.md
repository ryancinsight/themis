# Backlog — themis

Strategic roadmap; tags `[patch]`/`[minor]`/`[major]`/`[arch]` per SemVer class.
themis is the Atlas placement-law SSOT: typed, stateless vocabulary that
mnemosyne (allocation), moirai (scheduling), and hephaestus (devices) consume.

## THEMIS-AFFINITY-MASK-ACCESSOR-2026-09-01 [minor] — in progress

- **Integrator / lease:** Codex `01a0253c-6013-7552-99cc-36bbbcf77f6d` on
  `feat/group-aware-affinity-mask`; source lease discharged by provider commit
  `fde462944ae2dd936c9c8d3344c3e2f22f07e0a3`; lease now covers this item and
  delivery metadata; last update 2026-09-01.

- **Provider candidate:** `fde462944ae2dd936c9c8d3344c3e2f22f07e0a3`
  adds the cross-platform presence-proven efficiency view and Windows-only
  group-mask representation. It also registers the previously dormant CPU
  topology integration binary and moves embedded unit tests into a leaf,
  reducing `src/topology/cpu/mod.rs` from 686 to 464 lines. Exact-diff evidence:
  host warning-denied all-target/all-feature Clippy; 105/105 Nextest tests in
  1.684 s; no-default check; warning-denied x86_64 Linux and AArch64 Windows
  all-target/all-feature checks; warning-denied Rustdoc; 8/8 doctests; and
  cargo-semver-checks 196/196 under `[minor]`. `Cargo.lock` remains byte-identical
  to the standalone baseline. No throughput or allocation improvement is
  claimed for this cold topology-construction API.
- **Remaining closure:** publish and merge the provider, then migrate
  mnemosyne and hermes so the acceptance oracle below is exercised by real
  consumers before this item closes.

- **Evidence: three independent migrations, converging on the same two gaps.**
  apollo (#236), mnemosyne (#86), and hermes (#120) each moved off hand-rolled
  `GetLogicalProcessorInformationEx` walks onto themis in the same session,
  without coordinating. Two of the three independently re-derived identical
  knowledge that themis owns. That is a design signal, not three preferences.
- **1. Group-aware affinity mask (the substantive one).** themis numbers
  processors `group * 64 + bit`. Every thread-pinning consumer needs a
  platform affinity mask, so each must fold ids into one *and* know that
  convention to filter correctly:
  - mnemosyne folds with `checked_shl`, relying on the overflow as the group
    filter — a bit that will not fit a `usize` is a processor
    `SetProcessAffinityMask` cannot name.
  - hermes needs `SetThreadGroupAffinity`, binds the largest single-group
    share of a node, and reports the shortfall through a new
    `NumaBindingCoverage` because one call names one group.
  Both encode themis's numbering convention outside themis. If that
  convention ever changes, both break silently and neither has a test that
  would notice. A group-aware mask accessor — mask plus the group it belongs
  to, plus the processors that did not fit — keeps the knowledge where it is
  owned. **Note the two consumers need different Win32 calls, so the
  accessor must return group-partitioned data, not a single `usize`;** a
  flat mask would serve mnemosyne and leave hermes re-deriving.
- **2. Absence chaining (reported by all three).** `is_hybrid()` ->
  `highest_efficiency_class()` -> `processors_in_efficiency_class()` each
  return `Option`, but after `is_hybrid() == Some(true)` the rest cannot be
  `None`. mnemosyne's `performance_core_mask` carries two provably-dead
  `let-else` arms; apollo's `build()` discharges one underlying fact four
  times. Dead error channels are not free — they invite exactly the
  `None == None` confusion tracked in
  [[THEMIS-ABSENCE-EQUALITY-TRAP-2026-09-01]]. A presence witness whose
  methods are total would discharge absence once.
  mnemosyne also notes `detect() == None` and `efficiency_class_count() ==
  None` are distinct failures every consumer merges into one outcome anyway.
- **Acceptance oracle:** mnemosyne's `performance_core_mask` and hermes's
  `NumaBinding::bind` both reduce to the accessor with **no local knowledge
  of the numbering convention**, and hermes's multi-group coverage report is
  derived from what the accessor returns rather than recomputed. If either
  consumer still needs the convention afterwards, the accessor is wrong
  shape — fix it before landing, do not ship a partial.
- **Sequencing:** after the absence-equality trap, which is a correctness fix
  and should not wait behind ergonomics.

## THEMIS-ABSENCE-EQUALITY-TRAP-2026-09-01 [minor] — done 2026-09-01

- **Delivered** in `e12b795` (PR #37). `CpuTopology::is_in_highest_class(p) ->
  Option<bool>` returns `None` for both absence cases — classes unreported, or
  a processor outside the snapshot — and `Some(bool)` otherwise, with no panic
  path. Four tests on the existing hybrid/homogeneous fixtures; the regression
  oracle asserts the defective `==` spelling *still* fabricates before checking
  the predicate disagrees, so it cannot pass vacuously. Green on fmt/clippy/
  tests/doctests and MSRV 1.81 across ubuntu and windows, compile_fail codes,
  and miri. The three ergonomics findings were split out to
  [[THEMIS-AFFINITY-MASK-ACCESSOR-2026-09-01]] and are **not** closed by this.


- **Severity: this is the failure mode the crate exists to prevent, reachable
  through the most natural consumer spelling.** Surfaced by the apollo
  migration (apollo#236), not by a themis test.
- **The trap.** "Is processor `p` a performance core?" spells naturally as
  `processor_efficiency_class(p) == highest_efficiency_class()`. Both sides
  return `Option<EfficiencyClass>`. On a host that reports no efficiency
  classes both are `None`, and `None == None` is `true` — so the predicate
  answers **"yes, it is a performance core"** for *every* processor,
  including an out-of-range index. Absence silently becomes a confident
  positive. That is fabricated platform data arriving through `PartialEq`,
  precisely what the typed-absence discipline forbids.
- **Why this matters here specifically:** apollo's ADR 0042 was rewritten
  once already because a benchmark conclusion depended on which efficiency
  class a probe landed on. A predicate that reports every core as
  performance-class on an unreporting host reintroduces that exact inversion,
  silently, on any platform themis cannot classify.
- **Fix:** add a total predicate that cannot express the bug —
  `is_in_highest_class(&self, p) -> Option<bool>` — returning `None` for
  absence or an out-of-range processor, `Some(bool)` otherwise. Test on a
  no-class fixture that the naive `==` spelling and the predicate **disagree**
  (that disagreement is the regression oracle), and on an out-of-range index.
- **Consider alongside (same migration, lower severity, all from apollo#236):**
  - **No class→label rendering.** `EfficiencyClass` exposes only `rank()`, so
    each consumer re-derives "top/bottom/middle" and independently invents a
    word for the single-class host (apollo chose `uniform`). A `describe_class`
    upstream prevents the next consumer disagreeing — this is the per-repo
    drift the migration exists to end.
  - **Two `?` for one fact.** `highest_efficiency_class()?` then
    `processors_in_efficiency_class(..)?` — the second absence is unreachable
    once the first returned `Some`, yet the shipped doc example carries that
    dead channel. A `fastest_processors()` inherent method would collapse it.
  - **Absence re-derived at six accessors.** A presence witness
    (`fn efficiency(&self) -> Option<EfficiencyView<'_>>` with total methods)
    would let a consumer discharge absence once instead of per call; apollo's
    `build()` discharges the same underlying fact four times.
- **Sequencing:** the predicate is the correctness fix and lands alone. The
  other three are ergonomics and should not delay it.

## THEMIS-PLACEMENT-AXES-2026-09-01 [minor] — axis 1 (SMT siblings) delivered; axes 2 and 3 todo

- **Integrator / lease (2026-09-01):** Claude session d49f3b0a on
  `feat/themis-smt-siblings`, axis 1 only per the sequencing rule. Lease:
  `src/topology/cpu/{smt.rs,smt_view.rs,mod.rs,types.rs}`,
  `src/topology/cpu/efficiency/{records.rs,windows.rs}` (the
  `RelationProcessorCore` walk grows a core ordinal; the buffer producer is
  shared), `tests/topology/cpu.rs`, `docs/adr/0005-*`, CHANGELOG, this entry.
  Axes 2 and 3 stay todo.
- **Axis 1 delivered (2026-09-01, Claude):** `CpuTopology::smt()` /
  `CpuSmtView` over a per-processor `CoreId` table (ADR 0005). Windows reads
  it from the `RelationProcessorCore` walk the efficiency class already does
  (the walker now carries a core ordinal; one walk, two axes); Linux from
  `thread_siblings_list` with symmetry and coverage checks. A host without SMT
  is a present one-per-core table; absence is typed. Fixtures: the recorded
  24-processor hybrid host and the 8x2 homogeneous host, out-of-order records,
  double-claimed processors, gapped coverage, asymmetric lists. Gate: fmt,
  Clippy `-D warnings` with `testing` and with `--no-default-features`,
  nextest 118/118, 8/8 doctests, Rustdoc `-D warnings`. Consumer adoption in
  the apollo and mnemosyne pinned instruments is their items; lease released.
- **Context:** efficiency class (PR #35) closed one asymmetry axis. Three
  others exist in hardware, are not modelled, and are the same shape — an
  axis the OS reports that consumers must otherwise hardcode or guess. Filed
  together because they share one question: "are these two processors
  interchangeable for placement?"
- **1. SMT siblings (verified absent).** Nothing groups logical processors by
  physical core. Two SMT siblings are not interchangeable with two distinct
  cores: a compute-bound worker pool that treats them as equal oversubscribes
  by 2x, and a measurement instrument pinning to a sibling of a busy core
  measures contention. Windows reports this in the same
  `RelationProcessorCore` records the efficiency backend already walks (the
  `GROUP_AFFINITY` mask has multiple bits set for an SMT core); Linux exposes
  `cpuN/topology/thread_siblings_list`. This is the cheapest of the three and
  the most likely to be silently wrong today.
- **2. Favoured cores within a class.** Intel Turbo Boost Max 3.0 and AMD
  preferred cores bin one or two cores faster than their same-class peers.
  A "give me a performance processor" answer that ignores this is right about
  the class and wrong about the best member.
- **3. Last-level-cache domains.** AMD CCX/CCD and Intel sub-NUMA clustering
  create latency asymmetry WITHOUT class or NUMA-node asymmetry, so neither
  `efficiency_class` nor `numa_nodes` separates them. Cross-domain thread
  placement costs real bandwidth for shared-state work.
- **Non-goals:** GPU/NPU device placement — different abstraction, owned by
  hephaestus's `ComputeDevice` seam, not a CPU topology query.
- **Discipline (binding):** every axis follows `cache_levels()`/efficiency
  class — typed absence when the platform does not report it, no inference
  from core counts or model strings, homogeneous as a first-class reported
  result, parsers tested against recorded fixtures rather than this host.
- **Sequencing:** do not build all three speculatively. Each lands when a
  consumer needs it; SMT has the clearest present consumer (the pinned
  measurement instruments in apollo and mnemosyne, which currently cannot
  tell a sibling from a core).

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

## Core efficiency class [minor]

- [x] [minor] Model the performance/efficiency (P/E) distinction in
  `CpuTopology`. `EfficiencyClass` is a dense ordinal — higher is more
  performant, so three-tier parts are representable — reached through
  `efficiency_classes`, `processor_efficiency_class`, `efficiency_class_count`,
  `is_hybrid`, `highest_efficiency_class`, and `processors_in_efficiency_class`.
  Windows reads the `EfficiencyClass` byte of each
  `GetLogicalProcessorInformationEx(RelationProcessorCore)` record, Linux the
  Intel hybrid CPU-type `cpulist`s and then ARM `cpu_capacity`; every other
  target, and any host whose data does not cover every logical processor,
  reports typed absence. Retires the machine-specific performance-core
  constants hand-rolled in apollo's pinned probes and mnemosyne's benchmark
  harness, both of which mislabelled the developer host: its performance cores
  are the non-contiguous mask `0xc03c03`, so the probes pinned to cpu 2, an
  efficiency core. Decision:
  [ADR 0004](docs/adr/0004-efficiency-class-is-an-ordinal.md). Evidence: pure
  parsers over recorded platform bytes and sysfs strings, compiled in both
  backend targets' test builds, with fixtures pinning the `0xc03c03` host end
  to end, a homogeneous host, a three-tier host, sparse class bytes, fully and
  partially enumerated multi-group hosts, and malformed, truncated, empty and
  relationless buffers; live detection on the developer host returns exactly
  `{0, 1, 10, 11, 12, 13, 22, 23}`. Gates: fmt, warning-denied Clippy across
  `testing`/`no-default-features`/`all-features` on four targets, nextest
  `81/81`, doctests `6/6`, Rustdoc.

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
