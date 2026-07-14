//! Bounded Linux/Windows CPU-list parsing shared by topology detectors.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

const MAX_PROCESSOR_ID: usize = 32_768;

pub(crate) fn parse_cpu_list(cpulist: &str) -> Vec<u32> {
    let mut processors = Vec::new();
    for part in cpulist.trim().split(',') {
        if processors.len() >= MAX_PROCESSOR_ID {
            break;
        }
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (start.parse::<usize>(), end.parse::<usize>()) {
                if start < MAX_PROCESSOR_ID && end < MAX_PROCESSOR_ID && start <= end {
                    let limit = MAX_PROCESSOR_ID - start;
                    let remaining = MAX_PROCESSOR_ID - processors.len();
                    let count = limit.min(end - start + 1).min(remaining);
                    processors.extend(
                        (start..start + count)
                            .filter_map(|processor| u32::try_from(processor).ok()),
                    );
                }
            }
        } else if let Ok(processor) = part.parse::<u32>() {
            if usize::try_from(processor)
                .ok()
                .is_some_and(|processor| processor < MAX_PROCESSOR_ID)
            {
                processors.push(processor);
            }
        }
    }
    processors
}
