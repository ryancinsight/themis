//! Windows processor-group affinity representation.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

const PROCESSORS_PER_GROUP: u32 = 64;

/// One native affinity mask within a Windows processor group.
///
/// Themis numbers Windows logical processors as `group * 64 + bit`. This type
/// owns that convention so consumers never need to repeat the division,
/// remainder, or checked-shift arithmetic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProcessorGroupAffinity {
    group: u16,
    mask: usize,
}

impl ProcessorGroupAffinity {
    /// Represents one flattened logical processor as a native group mask.
    ///
    /// Returns `None` when the processor group does not fit `u16`, or when its
    /// bit cannot be represented by the target's native pointer-width mask.
    #[must_use]
    pub fn from_processor(processor: u32) -> Option<Self> {
        let group = u16::try_from(processor / PROCESSORS_PER_GROUP).ok()?;
        let bit = processor % PROCESSORS_PER_GROUP;
        let mask = 1usize.checked_shl(bit)?;
        Some(Self { group, mask })
    }

    /// Returns the operating-system processor group.
    #[must_use]
    pub const fn group(self) -> u16 {
        self.group
    }

    /// Returns the nonzero native affinity mask within [`Self::group`].
    #[must_use]
    pub const fn mask(self) -> usize {
        self.mask
    }

    /// Returns the number of processors represented by this mask.
    #[must_use]
    pub const fn processor_count(self) -> u32 {
        self.mask.count_ones()
    }
}

/// Group-partitioned Windows affinity masks for a logical-processor set.
///
/// Groups are sorted by ascending operating-system group id. Duplicate input
/// processors collapse to one mask bit. Processors that cannot be represented
/// by the target's native mask remain available through
/// [`Self::unassigned_processors`] instead of being silently discarded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessorAffinityGroups {
    groups: Box<[ProcessorGroupAffinity]>,
    unassigned_processors: Box<[u32]>,
}

impl ProcessorAffinityGroups {
    /// Partitions flattened logical processors into native affinity masks.
    ///
    /// # Examples
    ///
    /// ```
    /// use themis::ProcessorAffinityGroups;
    ///
    /// let affinity = ProcessorAffinityGroups::from_processors([0, 2, 64, 66]);
    /// assert_eq!(affinity.groups().len(), 2);
    /// assert_eq!(affinity.groups()[0].group(), 0);
    /// assert_eq!(affinity.groups()[0].mask(), 0b0101);
    /// assert_eq!(affinity.groups()[1].group(), 1);
    /// assert_eq!(affinity.groups()[1].mask(), 0b0101);
    /// assert!(affinity.unassigned_processors().is_empty());
    /// ```
    #[must_use]
    pub fn from_processors(processors: impl IntoIterator<Item = u32>) -> Self {
        let mut groups: Vec<ProcessorGroupAffinity> = Vec::new();
        let mut unassigned_processors = Vec::new();

        for processor in processors {
            let Some(affinity) = ProcessorGroupAffinity::from_processor(processor) else {
                if let Err(position) = unassigned_processors.binary_search(&processor) {
                    unassigned_processors.insert(position, processor);
                }
                continue;
            };

            match groups.binary_search_by_key(&affinity.group, |candidate| candidate.group) {
                Ok(position) => groups[position].mask |= affinity.mask,
                Err(position) => groups.insert(position, affinity),
            }
        }

        Self {
            groups: groups.into_boxed_slice(),
            unassigned_processors: unassigned_processors.into_boxed_slice(),
        }
    }

    /// Returns the sorted, nonempty native group masks.
    #[must_use]
    pub fn groups(&self) -> &[ProcessorGroupAffinity] {
        &self.groups
    }

    /// Returns one processor group's mask, when represented.
    #[must_use]
    pub fn group(&self, group: u16) -> Option<ProcessorGroupAffinity> {
        self.groups
            .binary_search_by_key(&group, |candidate| candidate.group)
            .ok()
            .map(|position| self.groups[position])
    }

    /// Returns processors that do not fit the target's native group-mask form.
    #[must_use]
    pub fn unassigned_processors(&self) -> &[u32] {
        &self.unassigned_processors
    }

    /// Returns whether every distinct requested processor is represented.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unassigned_processors.is_empty()
    }

    /// Returns the number of distinct processors represented by group masks.
    #[must_use]
    pub fn assigned_processor_count(&self) -> usize {
        self.groups
            .iter()
            // A native mask has at most `usize::BITS` set bits, so its
            // population count is representable by `usize` on every target.
            .map(|group| group.processor_count() as usize)
            .sum()
    }

    /// Returns the number of distinct requested processors.
    #[must_use]
    pub fn requested_processor_count(&self) -> usize {
        self.assigned_processor_count() + self.unassigned_processors.len()
    }

    /// Returns the group representing the most requested processors.
    ///
    /// Ties select the lowest group id, making the result independent of input
    /// order and suitable for APIs that can bind only one group.
    #[must_use]
    pub fn largest_group(&self) -> Option<ProcessorGroupAffinity> {
        let mut groups = self.groups.iter().copied();
        let mut largest = groups.next()?;
        for candidate in groups {
            if candidate.processor_count() > largest.processor_count() {
                largest = candidate;
            }
        }
        Some(largest)
    }
}
