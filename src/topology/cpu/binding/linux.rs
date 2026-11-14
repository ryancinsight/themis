//! Linux backend: `sched_getaffinity` and `sched_setaffinity` on the calling
//! thread.

use core::ffi::c_ulong;
use core::mem::size_of;

use super::BindError;
use crate::topology::cpu::MAX_PROCESSOR_ID;

const WORD_BITS: usize = size_of::<c_ulong>() * 8;
const WORDS: usize = MAX_PROCESSOR_ID / WORD_BITS;
const MASK_BYTES: usize = WORDS * size_of::<c_ulong>();

// The kernel's `cpumask` layout is `unsigned long` words; 4 KiB covers the
// crate's 32 768-processor bound exactly.
const _: () = assert!(MASK_BYTES == 4096 && MAX_PROCESSOR_ID % WORD_BITS == 0);

/// The processors one `sched_setaffinity` call can express: an inline bitmap
/// of `unsigned long` words, bit `n` of word `w` naming processor
/// `w * WORD_BITS + n`.
#[derive(Clone, Eq, Hash, PartialEq)]
pub(super) struct Mask {
    words: [c_ulong; WORDS],
}

/// Word index and bit of a processor, or `None` when the id is not a `usize`.
fn locate(processor: u32) -> Option<(usize, c_ulong)> {
    let bit = usize::try_from(processor).ok()?;
    Some((bit / WORD_BITS, 1 << (bit % WORD_BITS)))
}

impl Mask {
    pub(super) fn from_processors(
        processors: impl Iterator<Item = u32>,
    ) -> Result<Self, BindError> {
        let mut words = [0; WORDS];
        for processor in processors {
            let (index, bit) = locate(processor).ok_or(BindError::OutOfRange { processor })?;
            *words
                .get_mut(index)
                .ok_or(BindError::OutOfRange { processor })? |= bit;
        }
        Ok(Self { words })
    }

    pub(super) fn contains(&self, processor: u32) -> bool {
        locate(processor)
            .is_some_and(|(index, bit)| self.words.get(index).is_some_and(|word| word & bit != 0))
    }

    pub(super) fn len(&self) -> u32 {
        self.words.iter().map(|word| word.count_ones()).sum()
    }

    pub(super) fn processors(&self) -> impl Iterator<Item = u32> + '_ {
        (0u32..)
            .take(MAX_PROCESSOR_ID)
            .filter(|processor| self.contains(*processor))
    }
}

/// Reads the `errno` the failed call just left in the thread's error slot.
fn errno_error() -> BindError {
    // SAFETY: `__errno_location` has no preconditions and returns the calling
    // thread's valid, aligned `errno` slot, which this thread alone writes.
    let code = unsafe { *libc::__errno_location() };
    BindError::Os { code }
}

/// Reports the processors the calling thread may currently run on.
pub(super) fn current() -> Result<Mask, BindError> {
    let mut words = [0; WORDS];
    // SAFETY: pid 0 names the calling thread. `words` is a live, initialized
    // allocation of exactly `MASK_BYTES` bytes, the `cpusetsize` passed, which
    // is the length of the data pointed to, not `size_of::<cpu_set_t>()`; the
    // pointer cast keeps `unsigned long` alignment, and the kernel writes only
    // within the bytes.
    let status = unsafe { libc::sched_getaffinity(0, MASK_BYTES, words.as_mut_ptr().cast()) };
    if status == 0 {
        Ok(Mask { words })
    } else {
        Err(errno_error())
    }
}

/// Confines the calling thread to `mask`; on `Err` its affinity is unchanged.
pub(super) fn apply(mask: &Mask) -> Result<(), BindError> {
    // SAFETY: pid 0 names the calling thread. `mask.words` is a live,
    // initialized allocation of exactly `MASK_BYTES` bytes, the `cpusetsize`
    // passed, the pointer cast keeps `unsigned long` alignment, and the kernel
    // retains nothing past the call.
    let status = unsafe { libc::sched_setaffinity(0, MASK_BYTES, mask.words.as_ptr().cast()) };
    if status == 0 {
        Ok(())
    } else {
        Err(errno_error())
    }
}
