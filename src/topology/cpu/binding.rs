//! Confining the calling thread to logical processors.
//!
//! Unlike the best-effort node binding a consumer may layer on top, these
//! primitives report every failure as a typed [`BindError`], so a scheduler can
//! refuse to start a worker whose binding did not take effect. Nothing here
//! allocates or panics, so a consumer's restore path can call it from `Drop`.

use core::fmt;

#[cfg(all(target_os = "linux", not(miri)))]
mod linux;
mod thread_affinity;
#[cfg(not(any(all(target_os = "linux", not(miri)), all(windows, not(miri)))))]
mod unsupported;
#[cfg(all(windows, not(miri)))]
mod windows;

#[cfg(all(target_os = "linux", not(miri)))]
use linux as sys;
#[cfg(not(any(all(target_os = "linux", not(miri)), all(windows, not(miri)))))]
use unsupported as sys;
#[cfg(all(windows, not(miri)))]
use windows as sys;

pub use thread_affinity::{AffinityChange, ThreadAffinity};

/// Why the calling thread could not be confined to a set of logical processors.
///
/// The variants partition the failure by who can act on it: the target has no
/// backend ([`Self::Unsupported`]), the set is not expressible
/// ([`Self::OutOfRange`], [`Self::SpansGroups`], [`Self::NoActiveProcessor`]),
/// or the operating system refused the request ([`Self::Os`]). Every `Err`
/// leaves the thread's affinity unchanged, except that a readback failing after
/// the kernel accepted the call re-applies the previous affinity first and, if
/// that is refused too, leaves the accepted affinity in force.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BindError {
    /// This target has no thread-binding backend, or the crate runs under an
    /// interpreter that cannot issue the call.
    Unsupported,
    /// The processor id is beyond what the target's affinity interface can
    /// name.
    OutOfRange {
        /// The requested flattened logical processor id.
        processor: u32,
    },
    /// The set lies in several Windows processor groups, and one call confines
    /// a thread to one group.
    SpansGroups {
        /// How many distinct groups the set names.
        groups: u32,
    },
    /// The set holds no processor the host can run on: it is empty, or on
    /// Windows names a group the host lacks or only inactive processors.
    NoActiveProcessor,
    /// The operating system rejected the request, for example because the
    /// processor is offline or outside the process's allowed set.
    Os {
        /// The raw operating-system error code: `GetLastError` on Windows,
        /// `errno` on Linux.
        code: i32,
    },
}

impl fmt::Display for BindError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unsupported => {
                formatter.write_str("thread binding is unsupported on this target")
            }
            Self::OutOfRange { processor } => write!(
                formatter,
                "logical processor {processor} is outside the range this target can bind"
            ),
            Self::SpansGroups { groups } => write!(
                formatter,
                "the processor set spans {groups} processor groups, and one call binds one group"
            ),
            Self::NoActiveProcessor => {
                formatter.write_str("the processor set names no active logical processor")
            }
            Self::Os { code } => write!(
                formatter,
                "the operating system refused the thread binding (error code {code})"
            ),
        }
    }
}

impl core::error::Error for BindError {}

/// Reports the processors the calling thread may currently run on.
///
/// On a Windows 11 or Server 2022 host with more than 64 processors, a thread
/// that was never bound spans every group but reports only its primary group,
/// so restoring that report narrows the thread to one group.
///
/// # Errors
///
/// - [`BindError::Unsupported`] when the target has no backend.
/// - [`BindError::Os`] with the raw error code when the query fails.
///
/// # Examples
///
/// ```
/// use themis::{current_thread_affinity, BindError};
///
/// match current_thread_affinity() {
///     Ok(affinity) => assert!(affinity.len() >= 1),
///     Err(error) => assert_eq!(error, BindError::Unsupported),
/// }
/// ```
pub fn current_thread_affinity() -> Result<ThreadAffinity, BindError> {
    sys::current().map(ThreadAffinity::from_mask)
}

/// Confines the calling thread to `target` and reports what changed.
///
/// The call succeeds only when the operating system accepted the new affinity,
/// so a caller that publishes a worker-to-processor assignment after `Ok`
/// publishes a fact. [`AffinityChange::effective`] is the affinity read back
/// and can be a proper subset of `target`. There is no guard: restoring is
/// `bind_current_thread_to(change.previous())`, which replaces any affinity set
/// in between, returns `Err` rather than panicking when the kernel refuses it,
/// and never allocates.
///
/// # Errors
///
/// - [`BindError::Unsupported`] when the target has no backend.
/// - [`BindError::NoActiveProcessor`] on Windows when `target` names a group
///   the host lacks or no active processor of its group.
/// - [`BindError::Os`] with the raw error code when the operating system
///   refuses, for example `EINVAL` on Linux when no requested processor is
///   online and permitted.
///
/// # Examples
///
/// A worker pins itself to one processor of its current set and restores the
/// set when done. Which processors the host permits is environmental, so the
/// example is not run:
///
/// ```no_run
/// use themis::{bind_current_thread_to, current_thread_affinity, ThreadAffinity};
///
/// let first = current_thread_affinity()?
///     .processors()
///     .next()
///     .expect("an affinity names a processor");
/// let change = bind_current_thread_to(&ThreadAffinity::from_processors([first])?)?;
/// // ... pinned work ...
/// bind_current_thread_to(change.previous())?;
/// # Ok::<(), themis::BindError>(())
/// ```
pub fn bind_current_thread_to(target: &ThreadAffinity) -> Result<AffinityChange, BindError> {
    let previous = sys::current()?;
    sys::apply(target.mask())?;
    match sys::current() {
        Ok(effective) => Ok(AffinityChange::new(
            ThreadAffinity::from_mask(previous),
            ThreadAffinity::from_mask(effective),
        )),
        // The readback error is the one reported. A refused restore leaves the
        // accepted affinity in force and adds nothing the caller can act on.
        Err(readback) => sys::apply(&previous).and(Err(readback)).or(Err(readback)),
    }
}

/// Confines the calling thread to exactly one logical processor.
///
/// `processor` uses the flattened numbering of [`crate::CpuTopology`]: on
/// Windows `group * 64 + bit`, on Linux the kernel's cpu number. The binding
/// lasts for the thread's life unless a later call replaces it. It is
/// [`ThreadAffinity::from_processors`] over one id followed by
/// [`bind_current_thread_to`], keeping their error contract.
///
/// # Errors
///
/// - [`BindError::Unsupported`] when the target has no backend.
/// - [`BindError::OutOfRange`] when the processor id cannot be named by the
///   target's affinity interface: a Windows processor group beyond `u16`, or a
///   Linux id at or past the crate's processor-id bound of 32 768.
/// - [`BindError::NoActiveProcessor`] on Windows for a group the host lacks or
///   an inactive processor.
/// - [`BindError::Os`] with the raw error code when the operating system
///   refuses, for example `EINVAL` on Linux for a processor that is offline or
///   outside the process's cpuset.
///
/// # Examples
///
/// A processor id no target can name is refused, and the calling thread is
/// left untouched:
///
/// ```
/// use themis::{bind_current_thread, BindError};
///
/// assert!(matches!(
///     bind_current_thread(u32::MAX),
///     Err(BindError::OutOfRange { processor: u32::MAX } | BindError::Unsupported)
/// ));
/// ```
///
/// A worker pins itself once at startup and refuses to run unbound. Which
/// processors the host permits is environmental, so the example is not run:
///
/// ```no_run
/// use themis::bind_current_thread;
///
/// let worker = std::thread::spawn(|| bind_current_thread(0));
/// worker.join().expect("worker panicked")?;
/// # Ok::<(), themis::BindError>(())
/// ```
pub fn bind_current_thread(processor: u32) -> Result<(), BindError> {
    bind_current_thread_to(&ThreadAffinity::from_processors([processor])?).map(|_change| ())
}

#[cfg(test)]
mod tests;
