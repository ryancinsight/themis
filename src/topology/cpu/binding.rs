//! Confining the calling thread to one logical processor.
//!
//! Unlike the best-effort node binding a consumer may layer on top, this
//! primitive reports every failure as a typed [`BindError`], so a scheduler can
//! refuse to start a worker whose binding did not take effect.

use core::fmt;

#[cfg(all(target_os = "linux", not(miri)))]
mod linux;
#[cfg(all(windows, not(miri)))]
mod windows;

/// Why the calling thread could not be confined to a logical processor.
///
/// The variants partition the failure by who can act on it: the target has no
/// backend ([`Self::Unsupported`]), the processor id is not addressable
/// ([`Self::OutOfRange`]), or the operating system refused the request
/// ([`Self::Os`]). In every case the thread keeps its previous affinity.
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
            Self::Os { code } => write!(
                formatter,
                "the operating system refused the thread binding (error code {code})"
            ),
        }
    }
}

impl std::error::Error for BindError {}

/// Confines the calling thread to exactly one logical processor.
///
/// `processor` uses the flattened numbering of [`crate::CpuTopology`]: on
/// Windows `group * 64 + bit`, on Linux the kernel's cpu number. The binding
/// lasts for the thread's life. There is no guard and no restore, because a
/// worker thread pins once at startup; a second call rebinds the thread.
///
/// The call succeeds only when the operating system accepted the new
/// affinity, so a caller that publishes a worker-to-processor assignment after
/// `Ok(())` publishes a fact, and a caller that receives `Err` may refuse to
/// start the worker.
///
/// # Errors
///
/// - [`BindError::Unsupported`] when the target has no backend.
/// - [`BindError::OutOfRange`] when the processor id cannot be named by the
///   target's affinity interface: a Windows processor group beyond `u16`, or a
///   Linux id at or past the crate's processor-id bound of 32 768.
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
    #[cfg(all(target_os = "linux", not(miri)))]
    {
        linux::bind_current_thread(processor)
    }
    #[cfg(all(windows, not(miri)))]
    {
        windows::bind_current_thread(processor)
    }
    #[cfg(not(any(all(target_os = "linux", not(miri)), all(windows, not(miri)))))]
    {
        let _ = processor;
        Err(BindError::Unsupported)
    }
}

/// Reads the error the failed call just left in the thread's error slot.
///
/// Must run immediately after the failing call, before any other system call
/// on the thread can overwrite it.
#[cfg(all(any(target_os = "linux", windows), not(miri)))]
fn last_os_error() -> BindError {
    BindError::Os {
        code: std::io::Error::last_os_error()
            .raw_os_error()
            .expect("invariant: last_os_error is built from a raw OS code"),
    }
}

#[cfg(test)]
mod tests;
