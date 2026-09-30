//! Query unit tests.

#[cfg(all(feature = "std", target_os = "linux", not(miri)))]
use super::current_processor;
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

#[cfg(all(feature = "std", target_os = "linux", not(miri)))]
#[test]
fn linux_query_reports_the_processor_a_pinned_thread_runs_on() {
    use crate::{bind_current_thread, CpuTopology};

    // A fresh thread, so the harness thread is never left pinned.
    std::thread::spawn(|| {
        let running = current_processor().expect("the getcpu syscall reports the running cpu");
        assert_eq!(bind_current_thread(running), Ok(()));

        // Pinned to one processor, every later report is that processor, and
        // its node is the node the sysfs topology assigns to it.
        for _ in 0..64 {
            assert_eq!(current_processor(), Some(running));
        }
        let node = try_current_numa_node().expect("a reported processor carries its node");
        if let Some(expected) =
            CpuTopology::detect().and_then(|topology| topology.processor_to_numa_node(running))
        {
            assert_eq!(node, expected);
        }
    })
    .join()
    .expect("invariant: the assertions run inside the closure and are joined here");
}
