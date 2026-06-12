//! Melinoe-backed branded placement scopes.

mod sync_region;
mod thread_local;

#[cfg(test)]
mod tests;

pub use sync_region::{sync_region_placement_scope, SyncRegionPlacement};
pub use thread_local::{thread_local_placement_scope, ThreadLocalPlacement};
