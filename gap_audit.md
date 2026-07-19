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

### Pre-existing defect: branded placement panics with `melinoe` feature

**Severity**: bug.

**Tests affected**: `cell_and_slice_reference_types_avoid_allocations`,
`const_cell_and_slice_reference_types_work`.

**Root cause**: `SafePlacement::cell_index` at `src/branded/region/placement.rs` panics
with `region_index 0 out of bounds for 0 region(s)` when `CpuTopology` has zero NUMA
nodes. Triggered when `CpuTopology::new_for_test` builds a topology with processor-to-node
entries that reference a non-existent node. This is pre-existing — the original
`src/branded/tests.rs` has the same panic when run with `melinoe` feature enabled;
tests were never run with `--features melinoe` (which is the default) at the crate level.

**Fix**: Not part of the rehome. Requires fixing the `split`/`split_with` paths or the
test topology construction in the branded module.
