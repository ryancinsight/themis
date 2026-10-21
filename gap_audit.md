# themis gap audit

## SMT reported partition (0.11.3) [patch]

The [unchanged baseline](https://github.com/ryancinsight/themis/commit/5595fc0a5fdae7d9c017a26023f8960b8f8511fb) accepts reciprocal but non-transitive
reports `["0,1", "0,1,2", "1,2"]` as `[0,0,1]`, contradicting ADR 0005.
Linux-target Miri reproduces this. Normalized sets now must equal every
referenced sibling set; together with self-membership this proves symmetry
and transitivity. Bounds, typed absence, parser behavior and dense ids remain.

The [repair item](backlog/themis-smt-reported-partition.md) tracks regression,
relation-oracle, independent-review and compatibility evidence. Mac-native runs
exclude the platform-gated SMT module; cross-target checks only compile it.
Linux-target Miri executes the fixtures under an interpreter, not a Linux kernel.
Task receipts preserve the original standalone scanner refusal. The registered
lane's trusted scanner passes; final delivery and CI gates remain required.

## Testing-helper visibility repair (0.11.2) [patch]

Current default's Mac testing feature enables outer exports while helper
symbols remain Linux/Windows-or-unit-test gated. The normal dependency build
therefore fails E0432; default and unit-only greens do not discharge it.
The repair adds the already-declared testing condition to both definitions and
their inner export. Its availability implication now holds on every target;
with testing/cfg(test) off, the production predicate is unchanged.

The unmodified public topology harness is red before repair and 17/17 after.
Mac joins the existing full-feature CI matrix. Source and locked API checks,
all 78 Mac tests, no-default controls and supported-target cross-checks pass.
All tests and external package versions are retained. Melinoe stays at 0.10.0;
its Git pin advances to current default after a source comparison proves that
only provider hooks and its own lockfile changed. No library source changed.
Independent review and lifecycle acceptance remain open. Existing source leases
and historical items below are preserved; this does not close their work.

A separate preexisting developer-toolchain limit is recorded in
[THEMIS-MSRV-CARGO-DEV-GRAPH](backlog/themis-msrv-cargo-dev-graph.md): Cargo 1.81
cannot fetch/build tests for the locked developer graph because of an edition 2024
dev manifest. The candidate library passes the actual Rust/Cargo 1.81
all-features check; the baseline fails E0432 there. Additional Rust 1.81
compiler-only library checks with current Cargo pass. A complete Cargo 1.81
developer test-gate pass is not claimed.

## Open risks

### GATE-FEATURE-GATED-HARNESS-2026-09-21 — a default-feature gate skips two harnesses

`branded` carries `required-features = ["melinoe", "testing"]` and
`cpu_topology` carries `testing`, so a local `cargo nextest run` without
them reports a clean 77/77 while CI, which passes `--features testing`,
runs 119 and was red for three days on a compile error in `branded`
([#54](https://github.com/ryancinsight/themis/pull/54)). The board's own
evidence line for THEMIS-AFFINITY-MASK-ACCESSOR records `77/77` for the
same reason, so the gap has been mistaken for a pass more than once.

The stack's `[patch]` overlay compounds it: under the atlas umbrella
melinoe resolves to the local tree, so even a feature-complete run there
cannot see a stale pin. Only a standalone resolution reproduces CI.

**Re-open trigger:** any local gate claim for this member that does not
name `--features testing` and does not run outside the overlay.

**Mechanization:** the member needs a committed gate script that both the
pre-push hook and CI invoke, so the feature set cannot diverge between
them (`engineering_gates`: one gate definition).

## Resolved in current branch

### REGION-MANIFEST-001 — branded region manifest contained implementation

The fetched provider default placed the complete `SyncRegionPlacement`
implementation and tests in `src/branded/region/mod.rs`, a 481-line module
manifest. The implementation now lives in `src/branded/region/scope.rs`, while
the manifest declares child modules and re-exports the public scope surface.
ADR 0003 records the decision and its rejected alternatives.

Evidence: provider conformance `manifest_implementation` decreases from 2 to
1 with all other Themis classes unchanged, nextest `25/25`, doctests `5/5`,
locked all-target check, warning-denied Clippy, format, and Rustdoc pass.
Re-open only if a later module move changes public paths or the conformance
class rises.

## Deferred items

### TREE-SRP-001 Phase 2 — test rehoming to `tests/` ✅

**Status**: Complete (corrected 2026-07-18 — see Resolution correction below).
All topology and branded tests moved to `tests/`:
- `tests/topology/cpu.rs` (from `src/topology/tests/cpu.rs`)
- `tests/branded.rs` (from `src/branded/tests.rs`)
- `tests/gpu.rs`, `tests/tpu.rs` (already at `tests/`)
- `src/topology/tests/` and `src/branded/tests.rs` deleted.

**Resolution** (as originally recorded): Added `#[cfg(test)] pub fn new_for_test`
constructor on `CpuTopology`; widened builder functions and distance constants
to `pub`; added `#[cfg(test)] pub use` re-exports in `src/lib.rs`.

**Resolution correction (2026-07-18, PR #10 `b8f8b87` merged `9677a47`)**: the
original Resolution was verified only via `cargo check --lib`. `cargo nextest
run` failed with 7 E0432/E0599 errors because `#[cfg(test)]` does NOT activate
for the lib when consumed as a regular dependency by integration tests (the
lib is built with `--cfg test=false` in that context). The peer's initial
"Status: Complete" claim above was a PM-vs-tree drift defect (anti-gaming
violation). The corrected Resolution adds a `testing` cargo feature (implies
`std`), gates `new_for_test` + the `build_*` table-builder re-exports under
`cfg(any(test, feature = "testing"))`, adds a `topology/mod.rs` re-export of
`cpu::tables::{build_*}` so `lib.rs`'s `pub use topology::{build_*}` resolves,
drops 2 unused imports from `tests/branded.rs`, and adds doc comments to the 4
`build_*` builders (now crate-root-visible; required by `#![deny(missing_docs)]`).
Verification at HEAD `9677a47`: `cargo nextest run --features testing` 36/36
green, default 21/21 green, clippy `--all-targets -D warnings` clean both modes,
`cargo check --lib --no-default-features --features testing` clean,
`cargo fmt --check` clean.

### Stale finding retired: branded placement zero-node panic (verified 2026-08-06)

The former panic report is no longer present in the current Themis source: the
referenced `SafePlacement::cell_index` implementation and the old in-source
branded test module were removed during the branded-placement rehome. The
current production split paths are `SyncRegionPlacement::split` and
`SyncRegionPlacement::split_with`, and they derive their permit count directly
from `CpuTopology::numa_nodes()`.

Verification against the current repository SSOT:

- `cargo nextest run --features melinoe,testing --test branded --no-fail-fast`:
  **15/15 passed**, including `cell_and_slice_reference_types_avoid_allocations`
  and `const_cell_and_slice_reference_types_work`.
- A source audit found no remaining `SafePlacement`, `cell_index`, or
  `region_index 0 out of bounds` implementation/message in `src/` or `tests/`.

No code fix or consumer-side workaround is required. Re-open only if a future
branded-placement change reintroduces a zero-node panic or a topology split can
produce a permit whose node ID is not present in the topology.

### ADR governance — generated index refresh

The two ADR source headers already use the canonical `Status: Accepted` token.
The tracked `docs/adr/README.md` table was stale and now matches those headers,
including the generator marker and canonical separator width. This is derived
documentation only; no decision content or provider contract changed. The
cross-repository status remains owned by Atlas `ATLAS-ADR-GOV-058`.
