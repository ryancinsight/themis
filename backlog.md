# Backlog — themis

Milestone ladder: [backlog/milestones.md](backlog/milestones.md) ([ADR 0007](docs/adr/0007-release-scope-1-0.md)).
themis is the Atlas placement-law SSOT: typed, stateless vocabulary that
mnemosyne (allocation), moirai (scheduling), and hephaestus (devices) consume.
Items are ordered by priority, then dependency; delivered work lives in git history.

<a id="themis-msrv-cargo-dev-graph"></a>
## THEMIS-MSRV-CARGO-DEV-GRAPH — State the developer Cargo policy [patch] — todo

- Outcome: README and gap_audit state one policy: `rust-version = "1.81.0"` binds the library a consumer compiles; tests, benches, examples and dev-dependencies build on the pinned toolchain (`rust-toolchain.toml`, 1.97.0) only.
- Evidence: `.github/workflows/msrv.yml` already runs `cargo check --locked --all-features` at 1.81.0 on Linux and Windows (green on ef8de1c0) and records that dev-dependencies do not set `rust-version`; Cargo 1.81 failing on edition-2024 `serde_spanned` 1.1.1 in the dev graph falls outside that contract. No blocker remains.
- Recommendation: adopt that policy. Rejected: holding dev-dependencies at Cargo-1.81-parsable releases, which stales the developer graph with no consumer contract behind it.
- Veto point: edition 2021 rests only on the 1.81 floor; moving to edition 2024 (Rust 1.85) is a separate floor decision for the release-scope ADR 0007.
- Acceptance: README "Standalone verification" names the policy and links `msrv.yml` instead of this block; the gap_audit `Old Cargo developer graph` section and this block are deleted.
- priority: verification; milestone: M-topology-contracts; needs: none.
- scope: `README.md`, `gap_audit.md`, `backlog.md`.
- Next step: rewrite the README sentence, delete the gap_audit section and this block.
- basis: ef8de1c0cd1199ef3fdd7426adc966453e04d81f.

<a id="themis-class-processors-doctest"></a>
## THEMIS-CLASS-PROCESSORS-DOCTEST — Discharge absence once in the class-iteration doctest [patch] — todo

- Outcome: the `CpuTopology::processors_in_efficiency_class` doctest and its "Pair with" sentence discharge the no-class absence once.
- Evidence: `src/topology/cpu/topology.rs:339-353` calls `highest_efficiency_class()?` then `processors_in_efficiency_class(fastest)?`; the second `?` cannot fire, because that method returns `None` only when `efficiency()` is `None` (`topology.rs:367`), which the first call already excluded. `CpuEfficiencyView::highest_class_processors` (`efficiency_view.rs:83`, fde4629) is the collapse the retired THEMIS-ABSENCE-EQUALITY-TRAP block proposed; the doctest predates it.
- Acceptance: the doctest selects the processor through `topology.efficiency()?.highest_class_processors().next()`, keeps the `None` assertion on the unreported fixture, and `cargo test --locked --doc --all-features` passes.
- priority: verification; milestone: M-topology-contracts; needs: none.
- scope: `src/topology/cpu/topology.rs`.
- Next step: rewrite the example and the sentence above it.
- basis: ef8de1c0cd1199ef3fdd7426adc966453e04d81f.

<a id="themis-efficiency-class-label"></a>
## THEMIS-EFFICIENCY-CLASS-LABEL — Own the efficiency-class label in Themis [minor] — todo

- Outcome: `CpuEfficiencyView` maps a class to one typed tier (performance, efficiency, intermediate, uniform for a single-class host), so consumers stop re-deriving the ranking words.
- Evidence: apollo holds two copies of that rendering, `crates/apollo-bench/src/measurement/processor.rs:264` and `crates/apollo-fft/src/application/execution/kernel/measurement_cores.rs` `fn label`, each inventing `"uniform"`; themis `src/` has no such method. Finding from apollo#236, previously carried only by the THEMIS-ABSENCE-EQUALITY-TRAP block.
- Acceptance: a typed enum (not a string) derived from `rank()` and `class_count()`, tested by value on the hybrid, three-tier and single-class fixtures; an apollo board item migrates both copies.
- priority: tightening; milestone: M-topology-contracts; needs: none.
- scope: `src/topology/cpu/efficiency_view.rs`, `src/topology/cpu/tests.rs`, `CHANGELOG.md`.
- Next step: define the enum against apollo's two call sites and file the apollo migration item.
- basis: ef8de1c0cd1199ef3fdd7426adc966453e04d81f.

<a id="themis-placement-axes-2026-09-01"></a>
## THEMIS-PLACEMENT-AXES-2026-09-01 — Report favoured cores and last-level-cache domains [minor] — blocked

- Outcome: `CpuTopology` reports axis 2 (favoured cores within an efficiency class: Turbo Boost Max 3.0, AMD preferred cores) and axis 3 (last-level-cache domains: AMD CCX/CCD, Intel sub-NUMA clustering), each in the shape ADR 0005 gives axis 1 (SMT siblings, delivered as `CpuTopology::smt()`).
- Blocker: no consumer has requested either axis; ADR 0005 builds each axis only "when a consumer needs it".
- Re-open trigger: a request from a named consumer (a stack member's board item or PR) that cites this ID and the call site needing the axis.
- Acceptance: per requested axis, parsers tested against recorded Windows and Linux fixtures; a homogeneous host is a reported result; absence is typed; ADR 0005 is extended or a sibling ADR is recorded.
- Non-goals: GPU/NPU placement (hephaestus `ComputeDevice`); inference from core counts or model strings; building an axis no consumer requested.
- priority: feature; milestone: M-placement-binding; needs: none.
- scope: `src/topology/cpu/`, `tests/topology/`, `docs/adr/`, `CHANGELOG.md`.
- Next step: on the trigger, claim and build only the requested axis.
- basis: ef8de1c0cd1199ef3fdd7426adc966453e04d81f.
