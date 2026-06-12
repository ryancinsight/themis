//! Query unit tests.

use super::{current_numa_node, refresh_current_numa_node, try_current_numa_node};
use crate::NumaNodeId;

#[test]
fn current_node_refreshes_to_cached_value() {
    let refreshed = refresh_current_numa_node();
    assert_eq!(current_numa_node(), refreshed);
}

#[test]
fn uncached_node_matches_defaulted_cached_node_when_reported() {
    if let Some(reported) = try_current_numa_node() {
        assert_eq!(current_numa_node(), reported);
    } else {
        assert_eq!(refresh_current_numa_node(), NumaNodeId::ZERO);
    }
}
