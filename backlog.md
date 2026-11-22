# Backlog — themis

<a id="themis-stable-qualification"></a>
## THEMIS-STABLE-QUALIFICATION — themis stable-release qualification [patch] — todo
- priority: verification; milestone: M-topology-contracts; basis: `4ec2fb99b191`.
- Outcome: Exercise the fallback topology path at the declared Rust floor. Follow the [release scope](docs/adr/0007-release-scope-1-0.md) and [milestone exits](backlog/milestones.md).
- Representative manifest `themis-topology 0.11.5`. Target 1.0.0.
- [ ] [patch] Add the missing Rust 1.81 build for the no-backend fallback in THEMIS-MSRV-FALLBACK-TARGET-2026-11-16. Exercise supported feature combinations and the single-node result on a fallback target.
- [ ] [patch] Qualify discovery and binding with a real consumer. Compare reported placement with host topology and assert the documented unsupported result where binding is unavailable.
- [ ] [major] Close M-placement-binding and verify the stable topology package, MSRV claim, platform matrix and registry-only resolution on the release candidate.
- [ ] [patch] Attach exact-revision evidence for the local exits and this member’s [Atlas qualification criteria](https://github.com/ryancinsight/atlas/blob/17b0124a7548ad81e817284afc5cb115c1d1bc78/docs/parity/repositories.json).

Milestone ladder: [backlog/milestones.md](backlog/milestones.md) ([ADR 0007](docs/adr/0007-release-scope-1-0.md)).
themis is the Atlas placement-law SSOT: typed, stateless vocabulary that
mnemosyne (allocation), moirai (scheduling), and hephaestus (devices) consume.
Items are ordered by priority, then dependency; delivered work lives in git history.

<a id="themis-msrv-fallback-target-2026-11-16"></a>
## THEMIS-MSRV-FALLBACK-TARGET-2026-11-16 — Compile the no-backend fallback at the 1.81 floor [patch] — todo

- Outcome: the fallbacks that every target other than Linux and Windows compiles (macOS included) are built at the declared `rust-version` 1.81.0, so a floor break there fails a check before a consumer meets it.
- Evidence: `msrv.yml` checks only `--all-features` on `ubuntu-latest` and `windows-latest` (`:58`, `:79`), and that configuration compiles none of three arms. `query_os` (`src/query/platform.rs:158`) compiles where the target is neither Linux nor Windows or `cfg(miri)` holds; `unsupported` (`src/topology/cpu/binding.rs:13`, `:20`) compiles under the same condition and only where `std` is on (`mod binding` is `std`-gated, `src/topology/cpu/mod.rs:5-6`); the `single_node` arm of `CpuTopology::detect` (`src/topology/cpu/detect/mod.rs:8-12`, `:29-35`) compiles where the target is neither OS or `std` is off. `--all-features` turns `std` on, so each run selects a native backend and none of the three arms is built at 1.81.0. They are built at 1.97.0 (`ci.yml` `verify` on `macos-latest`; the detect arm also by the `--no-default-features` clippy and nextest steps, `ci.yml:96`, `:106`), and the first two on nightly under Miri.
- Acceptance: today no `msrv.yml` job checks any target but the Linux and Windows runners (`matrix.os`, `:58`). After: a job at `RUSTUP_TOOLCHAIN` 1.81.0, whose `rustc --version` step prints 1.81.0, runs `cargo check --locked --all-features` for a non-Linux, non-Windows target (a `macos-latest` cell, or `rustup target add` plus `--target` on Linux) and is green on the PR head; the same job is red when a construct needing a newer rustc is added to each of the three arms in turn.
- priority: verification; milestone: M-topology-contracts; needs: none.
- scope: `.github/workflows/msrv.yml`, `src/query/platform.rs`, `src/topology/cpu/binding.rs`, `src/topology/cpu/detect/mod.rs`.
- Next step: add the target cell to the existing matrix, then prove it red against a construct newer than 1.81 in each of the three arms.
- basis: e230e7aed16129441115c0049cd3a94faf3b143f.

<a id="themis-nightly-locked-2026-11-16"></a>
## THEMIS-NIGHTLY-LOCKED-2026-11-16 — Pass --locked to the nightly cargo steps [patch] — todo

- Outcome: the nightly doctest and Miri jobs verify the dependency set `Cargo.lock` records, as the dependency-resolving steps of `ci.yml` and `msrv.yml` already do, instead of re-resolving when the lock and manifest disagree.
- Evidence: `.github/workflows/ci.yml:140` (`cargo +nightly test --doc`), `:168` (`cargo +nightly miri test --features testing --lib branded`) and `:171` (`cargo +nightly miri test --features testing --test branded`) carry no `--locked`. Every other cargo step that resolves dependencies carries it: the clippy, nextest, doctest and doc steps in `ci.yml` (`:92`, `:96`, `:99`, `:106`, `:111`, `:116`) and the check in `msrv.yml:79`. `cargo fmt --check` (`ci.yml:89`) and `cargo --version` (`msrv.yml:70`) carry none and resolve nothing.
- Acceptance: `git grep -n 'cargo +nightly' .github/workflows/ci.yml | grep -v -e '--locked'` prints three lines today and none after; the `doctest-error-codes` and `miri` jobs pass on the PR head.
- priority: verification; milestone: M-topology-contracts; needs: none.
- scope: `.github/workflows/ci.yml`.
- Next step: add `--locked` to the three commands.
- basis: e230e7aed16129441115c0049cd3a94faf3b143f.

<a id="themis-msrv-floor-adr-2026-11-16"></a>
## THEMIS-MSRV-FLOOR-ADR-2026-11-16 — Record the MSRV floor and edition in ADR 0007 [patch] — todo

- Outcome: the 1.0 release-scope ADR states the `rust-version` floor and the edition, and when either may rise, so the README's deferral to it ("raising the floor is a decision for the release-scope ADR") points at a record.
- Evidence: `README.md` (MSRV paragraph) defers to [ADR 0007](docs/adr/0007-release-scope-1-0.md); `git grep -n '1\.81\|edition' docs/adr/0007-release-scope-1-0.md` is empty. `Cargo.toml` declares `rust-version = "1.81.0"` and `edition = "2021"`; edition 2024 needs Rust 1.85 (README).
- Recommended option (veto point): keep the 1.81.0 floor and edition 2021 through 1.0; raise either only when a non-dev dependency requires it, recorded by a dated revision note. Rejected: raising to 1.85 and edition 2024 now, since no consumer or regular dependency need is on record and it narrows the consumer set at the release that fixes the contract.
- Acceptance: the ADR's Decision section states the floor, edition and raise trigger with one dated revision note; the `git grep` above is non-empty; README link and `docs/adr/README.md` index still resolve.
- priority: verification; milestone: M-topology-contracts; needs: none.
- scope: `docs/adr/0007-release-scope-1-0.md`, `README.md`.
- Next step: rewrite the ADR in place on the recommendation unless the user redirects.
- basis: e230e7aed16129441115c0049cd3a94faf3b143f.

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
