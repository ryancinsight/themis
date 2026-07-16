//! Melinoe-backed branded placement scopes.

mod region;
mod thread_local;

pub use region::cell::{
    ConstNumaPinnedCell, ConstNumaPinnedCellRef, ConstNumaPinnedSlice, ConstNumaPinnedSliceRef,
    ConstPinnedCell, ConstPinnedSlice, NumaPinnedCell, NumaPinnedCellRef, NumaPinnedSlice,
    NumaPinnedSliceRef, PinnedCell, PinnedSlice,
};
pub use region::placement::{ConstNumaNodePlacement, NumaNodePlacement};
pub use region::{sync_region_placement_scope, SyncRegionPlacement};
pub use thread_local::{
    thread_local_placement_scope, ConstThreadLocalNumaPlacement, ThreadLocalNumaPlacement,
    ThreadLocalPlacement,
};
