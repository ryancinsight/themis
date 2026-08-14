//! Thread-local NUMA-node query cache.

use super::platform::query_numa_node_or_default;
use crate::law::NumaNodeId;

#[cfg(feature = "std")]
melinoe::thread_cached! {
    pub(crate) mod cached_node: NumaNodeId;
}

/// Returns the cached NUMA node for the calling thread.
#[must_use]
#[inline]
pub fn current_numa_node() -> NumaNodeId {
    #[cfg(feature = "std")]
    {
        if let Some(node) = cached_node::get() {
            node
        } else {
            let node = query_numa_node_or_default();
            cached_node::set(node);
            node
        }
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
