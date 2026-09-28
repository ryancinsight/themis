//! Thread-portable branded placement scopes.

pub mod cell;
pub mod placement;
pub mod static_cell;

mod scope;
mod storage;

pub use scope::{sync_region_placement_scope, SyncRegionPlacement};
