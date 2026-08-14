# themis gap audit

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
