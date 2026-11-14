//! The processor sets one affinity call can express, and the change a binding
//! made.

use core::fmt;

use super::{sys, BindError};
#[cfg(all(windows, not(miri)))]
use crate::topology::cpu::ProcessorGroupAffinity;

/// A non-empty set of logical processors that one operating-system affinity
/// call can express.
///
/// Processors use the flattened numbering of [`crate::CpuTopology`]. On Linux a
/// set holds any ids below the crate's bound of 32 768; on Windows one call
/// confines a thread to a single processor group, so a set holds the
/// processors of one group. The value is a fixed-size bitmap (4 KiB on Linux,
/// one native group mask on Windows) and building it never allocates.
///
/// # Examples
///
/// ```
/// use themis::{BindError, ThreadAffinity};
///
/// // A set with no processor is refused where a backend exists.
/// assert!(matches!(
///     ThreadAffinity::from_processors([]),
///     Err(BindError::NoActiveProcessor | BindError::Unsupported)
/// ));
/// ```
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ThreadAffinity {
    mask: sys::Mask,
}

impl ThreadAffinity {
    /// Builds the set naming exactly `processors`, repeated ids collapsing.
    ///
    /// # Errors
    ///
    /// - [`BindError::Unsupported`] when the target has no binding backend.
    /// - [`BindError::OutOfRange`] for an id the target cannot name: a Windows
    ///   processor group beyond `u16`, or a Linux id at or past 32 768.
    /// - [`BindError::SpansGroups`] on Windows when the ids lie in more than
    ///   one processor group.
    /// - [`BindError::NoActiveProcessor`] when `processors` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use themis::{BindError, ThreadAffinity};
    ///
    /// // No target can name an id this large.
    /// assert!(matches!(
    ///     ThreadAffinity::from_processors([u32::MAX]),
    ///     Err(BindError::OutOfRange { processor: u32::MAX } | BindError::Unsupported)
    /// ));
    /// ```
    pub fn from_processors(processors: impl IntoIterator<Item = u32>) -> Result<Self, BindError> {
        let mask = sys::Mask::from_processors(processors.into_iter())?;
        if mask.len() == 0 {
            return Err(BindError::NoActiveProcessor);
        }
        Ok(Self { mask })
    }

    /// Builds the set naming the processors of one native group mask.
    #[cfg(all(windows, not(miri)))]
    #[must_use]
    pub fn from_group(group: ProcessorGroupAffinity) -> Self {
        Self {
            mask: sys::Mask::from_group(group),
        }
    }

    /// Reports whether `processor` is in the set.
    #[must_use]
    pub fn contains(&self, processor: u32) -> bool {
        self.mask.contains(processor)
    }

    /// Returns the number of processors in the set, which is at least one.
    #[must_use]
    #[expect(
        clippy::len_without_is_empty,
        reason = "a ThreadAffinity is never empty: construction refuses an empty set"
    )]
    pub fn len(&self) -> u32 {
        self.mask.len()
    }

    /// Iterates the processors in ascending order.
    pub fn processors(&self) -> impl Iterator<Item = u32> + '_ {
        self.mask.processors()
    }

    pub(super) fn from_mask(mask: sys::Mask) -> Self {
        Self { mask }
    }

    pub(super) fn mask(&self) -> &sys::Mask {
        &self.mask
    }
}

impl fmt::Debug for ThreadAffinity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.processors()).finish()
    }
}

/// What a successful [`crate::bind_current_thread_to`] changed.
///
/// `effective` is the affinity read back after the call. It can be a proper
/// subset of the request: Linux intersects a request with the processors the
/// host has online and the process's cpuset permits.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AffinityChange {
    previous: ThreadAffinity,
    effective: ThreadAffinity,
}

impl AffinityChange {
    pub(super) fn new(previous: ThreadAffinity, effective: ThreadAffinity) -> Self {
        Self {
            previous,
            effective,
        }
    }

    /// The affinity the thread had before the call, to restore with
    /// `bind_current_thread_to(change.previous())`.
    #[must_use]
    pub fn previous(&self) -> &ThreadAffinity {
        &self.previous
    }

    /// The affinity the thread has after the call.
    #[must_use]
    pub fn effective(&self) -> &ThreadAffinity {
        &self.effective
    }
}
