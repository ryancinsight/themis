//! Behaviour of thread binding, asserted against the operating system's own
//! report of the thread's affinity.
//!
//! Every binding runs on a spawned thread so the harness thread is never left
//! pinned.

use super::{bind_current_thread, BindError};

#[cfg(all(target_os = "linux", not(miri)))]
use super::linux::current_confinement;
#[cfg(all(windows, not(miri)))]
use super::windows::current_confinement;

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
fn on_fresh_thread<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::spawn(body)
        .join()
        .expect("invariant: the assertions run inside the closure and are joined here")
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn binding_confines_the_thread_to_exactly_the_requested_processor() {
    on_fresh_thread(|| {
        let allowed = current_confinement();
        let first = *allowed
            .first()
            .expect("a running thread is allowed somewhere");

        assert_eq!(bind_current_thread(first), Ok(()));
        assert_eq!(current_confinement(), vec![first]);

        // A second call moves the thread within the permitted set, which is how
        // a caller corrects a placement.
        let last = *allowed
            .last()
            .expect("a running thread is allowed somewhere");
        assert_eq!(bind_current_thread(last), Ok(()));
        assert_eq!(current_confinement(), vec![last]);
    });
}

#[cfg(all(any(target_os = "linux", windows), not(miri)))]
#[test]
fn a_refused_binding_leaves_the_thread_confinement_unchanged() {
    on_fresh_thread(|| {
        let allowed = current_confinement();

        assert!(matches!(
            bind_current_thread(refused_processor()),
            Err(BindError::Os { .. })
        ));
        assert_eq!(current_confinement(), allowed);
    });
}

#[cfg(all(target_os = "linux", not(miri)))]
fn refused_processor() -> u32 {
    // The last id below the crate bound is far past any online cpu: glibc
    // rejects a set bit beyond the kernel cpumask size with `EINVAL`, and a
    // kernel that accepts the width intersects the mask with the online set and
    // finds it empty, also `EINVAL`.
    u32::try_from(super::super::MAX_PROCESSOR_ID - 1).expect("invariant: the bound fits u32")
}

#[cfg(all(windows, not(miri)))]
fn refused_processor() -> u32 {
    // Group 65_535 is representable as a `GROUP_AFFINITY` but names no group
    // the host has, so the kernel refuses it rather than the mapping.
    65_535 * 64
}

#[cfg(all(target_os = "linux", not(miri)))]
#[test]
fn linux_reports_einval_for_a_processor_outside_the_cpuset() {
    const EINVAL: i32 = 22;
    on_fresh_thread(|| {
        assert_eq!(
            bind_current_thread(refused_processor()),
            Err(BindError::Os { code: EINVAL })
        );
    });
}

#[cfg(all(target_os = "linux", not(miri)))]
#[test]
fn linux_processor_at_the_bound_is_out_of_range_before_any_allocation() {
    let processor = u32::try_from(super::super::MAX_PROCESSOR_ID).expect("invariant: fits u32");
    assert_eq!(
        bind_current_thread(processor),
        Err(BindError::OutOfRange { processor })
    );
    assert_eq!(
        bind_current_thread(u32::MAX),
        Err(BindError::OutOfRange {
            processor: u32::MAX
        })
    );
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

#[cfg(any(miri, not(any(target_os = "linux", windows))))]
#[test]
fn a_target_without_a_backend_reports_unsupported() {
    assert_eq!(bind_current_thread(0), Err(BindError::Unsupported));
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
}
