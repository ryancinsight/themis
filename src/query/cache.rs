//! Thread-local NUMA-node query cache.

use super::platform::query_numa_node_or_default;
use crate::law::NumaNodeId;

#[cfg(feature = "std")]
melinoe::thread_cached! {
    /// Cached NUMA node for the calling thread.
    mod cached_node: NumaNodeId;
}

/// Returns the cached NUMA node for the calling thread.
#[must_use]
#[inline]
pub fn current_numa_node() -> NumaNodeId {
    #[cfg(feature = "std")]
    {
        cached_node::get_or_init(query_numa_node_or_default)
    }

    #[cfg(not(feature = "std"))]
    {
        NumaNodeId::ZERO
    }
}

/// Refreshes and returns the calling thread's NUMA node.
#[must_use]
#[inline]
pub fn refresh_current_numa_node() -> NumaNodeId {
    let node = query_numa_node_or_default();
    #[cfg(feature = "std")]
    {
        cached_node::set(node);
    }
    node
}
