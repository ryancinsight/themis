//! Query unit tests.

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
mod pin;

use super::{current_numa_node, refresh_current_numa_node, try_current_numa_node};
use crate::NumaNodeId;

#[test]
fn refresh_and_try_report_the_same_os_node() {
    on_pinned_thread(|| {
        let reported = try_current_numa_node();
        assert_eq!(
            refresh_current_numa_node(),
            reported.unwrap_or(NumaNodeId::ZERO)
        );
    });
}

// With `std`, the placement query is the thread's cached OS node; without it
// there is no cache and the query is node 0 whatever the OS reports (ADR 0006,
// Consequences).
#[cfg(feature = "std")]
#[test]
fn cached_node_follows_the_reported_node() {
    on_pinned_thread(|| {
        let reported = try_current_numa_node();
        assert_eq!(
            current_numa_node(),
            reported.unwrap_or(NumaNodeId::ZERO),
            "an unset cache is filled from the OS report"
        );

        let refreshed = refresh_current_numa_node();
        assert_eq!(refreshed, reported.unwrap_or(NumaNodeId::ZERO));
        assert_eq!(current_numa_node(), refreshed);
    });
}

#[cfg(not(feature = "std"))]
#[test]
fn placement_node_is_zero_whatever_the_os_reports() {
    on_pinned_thread(|| {
        let reported = try_current_numa_node();
        assert_eq!(
            refresh_current_numa_node(),
            reported.unwrap_or(NumaNodeId::ZERO),
            "a refresh still reads the OS"
        );
        assert_eq!(current_numa_node(), NumaNodeId::ZERO);
    });
}

/// Runs `check` on a fresh thread confined to one processor, so every probe
/// inside it reads one node and the harness thread is never left pinned.
fn on_pinned_thread(check: fn()) {
    std::thread::spawn(move || {
        #[cfg(all(any(target_os = "linux", windows), not(miri)))]
        if let Some(&processor) = pin::allowed_processors().first() {
            pin::pin_to(processor);
        }
        check();
    })
    .join()
    .expect("invariant: the assertions run inside the closure and are joined here");
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn query_reports_the_processor_a_pinned_thread_runs_on() {
    use super::current_processor;

    // A fresh thread, so the harness thread is never left pinned. The expected
    // processor comes from the OS affinity API, not from `current_processor`.
    std::thread::spawn(|| {
        let allowed = pin::allowed_processors();
        assert!(
            !allowed.is_empty(),
            "the OS reports the processors the thread may run on"
        );
        for processor in allowed {
            pin::pin_to(processor);
            for _ in 0..8 {
                assert_eq!(current_processor(), Some(processor));
            }
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
