# themis gap audit

## Deferred items

### TREE-SRP-001 Phase 2 — test rehoming to `tests/` ✅

**Status**: Complete. All topology and branded tests moved to `tests/`:
- `tests/topology/cpu.rs` (from `src/topology/tests/cpu.rs`)
- `tests/branded.rs` (from `src/branded/tests.rs`)
- `tests/gpu.rs`, `tests/tpu.rs` (already at `tests/`)
- `src/topology/tests/` and `src/branded/tests.rs` deleted.

**Resolution**: Added `#[cfg(test)] pub fn new_for_test` constructor on `CpuTopology`;
widened builder functions and distance constants to `pub`; added `#[cfg(test)] pub use`
re-exports in `src/lib.rs`.

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
