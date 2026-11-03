//! Bounded Linux/Windows CPU-list parsing shared by topology detectors.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use super::MAX_PROCESSOR_ID;
use core::ops::RangeInclusive;

pub(crate) fn parse_cpu_list(cpulist: &str) -> Vec<u32> {
    let mut processors = Vec::with_capacity(64);
    for part in cpulist.trim().split(',') {
        if processors.len() >= MAX_PROCESSOR_ID {
            break;
        }
        if let Some(range) = processor_range(part) {
            let remaining = MAX_PROCESSOR_ID - processors.len();
            processors.extend(range.take(remaining));
        }
    }
    processors
}

/// Parses a complete sysfs bitmap-list report, never its valid prefix.
///
/// Linux `bitmap_list_string` emits decimal ids/ranges in ascending disjoint
/// order. Each id is below the common allocation bound. These invariants imply
/// that the result has at most `MAX_PROCESSOR_ID` elements without truncation.
/// An invalid token, overlap, reversal, or out-of-bound id invalidates the set.
/// Outer whitespace accommodates sysfs's terminating newline.
///
/// Source: Linux v6.12 `lib/vsprintf.c`, `bitmap_list_string`.
pub(super) fn parse_cpu_set(cpulist: &str) -> Option<Vec<u32>> {
    let mut processors = Vec::with_capacity(64);
    for part in cpulist.trim().split(',') {
        if !part
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'-')
        {
            return None;
        }
        let range = processor_range(part)?;
        if processors.last().is_some_and(|last| last >= range.start()) {
            return None;
        }
        processors.extend(range);
    }
    Some(processors)
}

/// The shared bounded numeric decoder; callers select tolerant or complete-set
/// semantics. Rust's optional leading `+` remains accepted by the tolerant
/// list reader; the complete-set reader checks decimal spelling first.
fn processor_range(part: &str) -> Option<RangeInclusive<u32>> {
    let (start, end) = part.split_once('-').unwrap_or((part, part));
    let start = start.parse::<u32>().ok()?;
    let end = end.parse::<u32>().ok()?;
    (start <= end && usize::try_from(end).ok()? < MAX_PROCESSOR_ID).then_some(start..=end)
}

#[cfg(test)]
mod tests;
