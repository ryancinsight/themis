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

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn query_reports_the_processor_a_pinned_thread_runs_on() {
    use super::current_processor;
    use crate::{bind_current_thread, current_thread_affinity};

    // A fresh thread, so the harness thread is never left pinned.
    std::thread::spawn(|| {
        let running = current_processor().expect("the OS reports the running processor");
        let allowed = current_thread_affinity().expect("a running thread reports its affinity");
        assert!(allowed.contains(running));
        assert_eq!(bind_current_thread(running), Ok(()));

        // Pinned to one processor, every later report is that processor.
        for _ in 0..64 {
            assert_eq!(current_processor(), Some(running));
        }
    })
    .join()
    .expect("invariant: the assertions run inside the closure and are joined here");
}

#[cfg(all(feature = "std", target_os = "linux", not(miri)))]
#[test]
fn linux_query_reports_the_node_the_topology_assigns_to_a_pinned_processor() {
    use super::current_processor;
    use crate::{bind_current_thread, CpuTopology};

    std::thread::spawn(|| {
        let running = current_processor().expect("the getcpu syscall reports the running cpu");
        assert_eq!(bind_current_thread(running), Ok(()));

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
