//! Typed placement law for Atlas memory and runtime crates.
//!
//! Themis owns placement vocabulary, not allocation or execution. Mnemosyne
//! consumes these types to choose memory locality. Moirai consumes these types
//! to choose worker and task locality.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(nightly_tls_active, feature(thread_local))]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

mod law;
mod query;
mod topology;

#[cfg(feature = "melinoe")]
mod branded;

#[cfg(feature = "melinoe")]
pub use branded::{
    sync_region_placement_scope, thread_local_placement_scope, ConstNumaNodePlacement,
    ConstNumaPinnedCell, ConstNumaPinnedCellRef, ConstNumaPinnedSlice, ConstNumaPinnedSliceRef,
    ConstPinnedCell, ConstPinnedSlice, ConstThreadLocalNumaPlacement, NumaNodePlacement,
    NumaPinnedCell, NumaPinnedCellRef, NumaPinnedSlice, NumaPinnedSliceRef, PinnedCell,
    PinnedSlice, SyncRegionPlacement, ThreadLocalNumaPlacement, ThreadLocalPlacement,
};
pub use law::{
    LocalityDomainId, MemoryTier, NumaBucketIndex, NumaNodeId, PlacementHint, TopologyEpoch,
    WorkerId,
};
pub use query::{
    current_numa_node, current_processor, refresh_current_numa_node, try_current_numa_node,
};
pub use topology::{
    CacheLevel, CpuTopology, GpuDeviceProperties, GpuTopology, NumaNode, TpuDeviceProperties,
    TpuTopology,
};

// Test-only re-exports for integration tests (they are `pub` at definition
// site but live inside a private `mod topology`, so crate-root re-export
// is required for integration-test access). Gated on `feature = "testing"`
// (not just `cfg(test)`) because integration tests in `tests/` consume the
// lib as a regular dependency; `cfg(test)` only activates when the lib itself
// is the test target, not when it is depended on.
#[cfg(any(test, feature = "testing"))]
pub use topology::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
};
