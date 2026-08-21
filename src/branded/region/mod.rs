//! Thread-portable branded placement scopes.

pub mod cell;
pub mod placement;
pub mod static_cell;

mod scope;

pub use scope::{sync_region_placement_scope, SyncRegionPlacement};
