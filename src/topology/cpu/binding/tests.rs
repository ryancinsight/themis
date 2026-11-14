//! Behaviour of thread binding, asserted against the operating system's own
//! report of the thread's affinity through [`current_thread_affinity`].
//!
//! Every binding runs on a spawned thread so the harness thread is never left
//! pinned.

use std::string::ToString;
use std::vec::Vec;
use std::{format, vec};

use super::{
    bind_current_thread, bind_current_thread_to, current_thread_affinity, BindError, ThreadAffinity,
};

fn set(processors: impl IntoIterator<Item = u32>) -> ThreadAffinity {
    ThreadAffinity::from_processors(processors).expect("invariant: the fixture ids are bindable")
}

fn ids(affinity: &ThreadAffinity) -> Vec<u32> {
    affinity.processors().collect()
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
fn on_fresh_thread<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::spawn(body)
        .join()
        .expect("invariant: the assertions run inside the closure and are joined here")
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
fn allowed() -> Vec<u32> {
    ids(&current_thread_affinity().expect("a running thread reports its affinity"))
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn binding_confines_the_thread_to_exactly_the_requested_processor() {
    on_fresh_thread(|| {
        let allowed = allowed();
        let first = *allowed
            .first()
            .expect("a running thread is allowed somewhere");

        assert_eq!(bind_current_thread(first), Ok(()));
        assert_eq!(allowed(), vec![first]);

        // A second call moves the thread within the permitted set, which is how
        // a caller corrects a placement.
        let last = *allowed
            .last()
            .expect("a running thread is allowed somewhere");
        assert_eq!(bind_current_thread(last), Ok(()));
        assert_eq!(allowed(), vec![last]);
    });
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn set_binding_round_trips() {
    on_fresh_thread(|| {
        let before = current_thread_affinity().expect("a running thread reports its affinity");
        let pair: Vec<u32> = before.processors().take(2).collect();
        assert_eq!(pair.len(), 2, "the host allows this thread two processors");
        let requested = set(pair.iter().copied());

        let change = bind_current_thread_to(&requested).expect("two permitted processors bind");
        assert_eq!(change.previous(), &before);
        assert_eq!(change.effective(), &requested);
        assert_eq!(
            current_thread_affinity().expect("a bound thread reports its affinity"),
            requested
        );

        // Restoring is one more bind of `previous`, which replaces what was set.
        let restored = bind_current_thread_to(change.previous()).expect("the previous set binds");
        assert_eq!(restored.previous(), &requested);
        assert_eq!(restored.effective(), &before);
        assert_eq!(
            current_thread_affinity().expect("a restored thread reports its affinity"),
            before
        );
    });
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn a_refused_binding_leaves_the_thread_confinement_unchanged() {
    on_fresh_thread(|| {
        let before = allowed();

        assert_eq!(bind_current_thread(refused_processor()), Err(refusal()));
        assert_eq!(allowed(), before);
    });
}

#[cfg(all(target_os = "linux", not(miri)))]
fn refused_processor() -> u32 {
    // The last id below the crate bound is far past any online cpu: glibc
    // rejects a set bit beyond the kernel cpumask size with `EINVAL`, and a
    // kernel that accepts the width intersects the mask with the online set and
    // finds it empty, also `EINVAL`.
    32_767
}

#[cfg(all(target_os = "linux", not(miri)))]
fn refusal() -> BindError {
    const EINVAL: i32 = 22;
    BindError::Os { code: EINVAL }
}

#[cfg(all(windows, not(miri)))]
fn refused_processor() -> u32 {
    // Group 65_535 is representable as a `GROUP_AFFINITY` but is no group the
    // host has, so narrowing to the group's active processors leaves nothing.
    65_535 * 64
}

#[cfg(all(windows, not(miri)))]
fn refusal() -> BindError {
    BindError::NoActiveProcessor
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn a_set_names_its_processors_once_and_in_ascending_order() {
    let affinity = set([5, 3, 5]);
    assert_eq!(ids(&affinity), vec![3, 5]);
    assert_eq!(affinity.len(), 2);
    assert!(affinity.contains(3) && affinity.contains(5));
    assert!(!affinity.contains(4) && !affinity.contains(u32::MAX));
    assert_eq!(format!("{affinity:?}"), "[3, 5]");

    // Processors 67 and 69 are group 1, bits 3 and 5, on Windows.
    assert_eq!(ids(&set([69, 67])), vec![67, 69]);
    assert_eq!(set([3, 5]), set([5, 3]));
    assert_ne!(set([3, 5]), set([3]));
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn an_empty_set_names_no_active_processor() {
    assert_eq!(
        ThreadAffinity::from_processors([]),
        Err(BindError::NoActiveProcessor)
    );
}

#[cfg(all(target_os = "linux", not(miri)))]
#[test]
fn linux_processor_at_the_bound_is_out_of_range_before_any_binding() {
    for processor in [32_768, u32::MAX] {
        assert_eq!(
            bind_current_thread(processor),
            Err(BindError::OutOfRange { processor })
        );
        assert_eq!(
            ThreadAffinity::from_processors([0, processor]),
            Err(BindError::OutOfRange { processor })
        );
    }
}

#[cfg(all(windows, not(miri)))]
#[test]
fn windows_group_beyond_u16_is_out_of_range() {
    for processor in [65_536 * 64, u32::MAX] {
        assert_eq!(
            bind_current_thread(processor),
            Err(BindError::OutOfRange { processor })
        );
    }
}

#[cfg(all(windows, not(miri)))]
#[test]
fn windows_set_in_several_groups_reports_its_group_count() {
    assert_eq!(
        ThreadAffinity::from_processors([0, 64]),
        Err(BindError::SpansGroups { groups: 2 })
    );
    // Repeated and unordered ids count each group once.
    assert_eq!(
        ThreadAffinity::from_processors([130, 1, 64, 2, 131, 65]),
        Err(BindError::SpansGroups { groups: 3 })
    );
}

#[cfg(all(windows, not(miri)))]
#[test]
fn windows_from_group_names_the_processors_of_the_native_mask() {
    use crate::ProcessorAffinityGroups;

    let groups = ProcessorAffinityGroups::from_processors([0, 2, 64]);
    let first = ThreadAffinity::from_group(groups.groups()[0]);
    assert_eq!(ids(&first), vec![0, 2]);
    assert_eq!(first, set([2, 0]));
}

#[cfg(any(miri, not(any(target_os = "linux", windows))))]
#[test]
fn a_target_without_a_backend_reports_unsupported() {
    assert_eq!(bind_current_thread(0), Err(BindError::Unsupported));
    assert_eq!(
        ThreadAffinity::from_processors([0]),
        Err(BindError::Unsupported)
    );
    assert_eq!(current_thread_affinity(), Err(BindError::Unsupported));
}

#[test]
fn errors_render_their_payload() {
    assert_eq!(
        BindError::OutOfRange { processor: 7 }.to_string(),
        "logical processor 7 is outside the range this target can bind"
    );
    assert_eq!(
        BindError::Os { code: 22 }.to_string(),
        "the operating system refused the thread binding (error code 22)"
    );
    assert_eq!(
        BindError::Unsupported.to_string(),
        "thread binding is unsupported on this target"
    );
    assert_eq!(
        BindError::SpansGroups { groups: 2 }.to_string(),
        "the processor set spans 2 processor groups, and one call binds one group"
    );
    assert_eq!(
        BindError::NoActiveProcessor.to_string(),
        "the processor set names no active logical processor"
    );
}
