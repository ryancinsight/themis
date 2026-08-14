//! Thread-local NUMA-node query cache.

use super::platform::query_numa_node_or_default;
use crate::law::NumaNodeId;

#[cfg(all(feature = "std", nightly_tls_active))]
#[thread_local]
static CACHED_NODE: core::cell::Cell<Option<NumaNodeId>> = const { core::cell::Cell::new(None) };

#[cfg(all(feature = "std", not(nightly_tls_active)))]
thread_local! {
    // The initializer is already a `const` block; retain the explicit
    // expectation because Clippy's target-specific diagnostic is redundant.
    #[expect(
        clippy::missing_const_for_thread_local,
        reason = "false positive: the initializer is already a const block"
    )]
    static CACHED_NODE: core::cell::Cell<Option<NumaNodeId>> = const { core::cell::Cell::new(None) };
}

/// Returns the cached NUMA node for the calling thread.
#[must_use]
#[inline]
pub fn current_numa_node() -> NumaNodeId {
    #[cfg(feature = "std")]
    {
        #[cfg(nightly_tls_active)]
        {
            if let Some(node) = CACHED_NODE.get() {
                node
            } else {
                let node = query_numa_node_or_default();
                CACHED_NODE.set(Some(node));
                node
            }
        }
        #[cfg(not(nightly_tls_active))]
        {
            CACHED_NODE.with(|cell| {
                if let Some(node) = cell.get() {
                    node
                } else {
                    let node = query_numa_node_or_default();
                    cell.set(Some(node));
                    node
                }
            })
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
        #[cfg(nightly_tls_active)]
        {
            CACHED_NODE.set(Some(node));
        }
        #[cfg(not(nightly_tls_active))]
        {
            CACHED_NODE.with(|cell| cell.set(Some(node)));
        }
    }
    node
}
