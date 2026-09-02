//! Borrowed, presence-proven SMT sibling queries.

use super::CpuTopology;
#[cfg(windows)]
use super::ProcessorAffinityGroups;
use crate::topology::types::CoreId;

/// A borrowed core table whose presence is already proven.
///
/// Construct this view through [`CpuTopology::smt`]. Its table-level queries
/// are total because the optional platform report has been discharged once at
/// the boundary. Processor-index queries remain optional because an index can
/// lie outside the snapshot.
#[derive(Clone, Copy, Debug)]
pub struct CpuSmtView<'topology> {
    cores: &'topology [CoreId],
    core_count: usize,
}

impl<'topology> CpuSmtView<'topology> {
    pub(super) fn new(cores: &'topology [CoreId]) -> Option<Self> {
        let highest = cores.iter().max().copied()?;
        Some(Self {
            cores,
            core_count: usize::try_from(highest.get()).ok()? + 1,
        })
    }

    /// Returns the per-processor core ids.
    #[must_use]
    pub const fn cores(self) -> &'topology [CoreId] {
        self.cores
    }

    /// Returns how many physical cores the snapshot's processors occupy.
    #[must_use]
    pub const fn physical_core_count(self) -> usize {
        self.core_count
    }

    /// Returns whether any core carries more than one logical processor.
    #[must_use]
    pub const fn has_smt(self) -> bool {
        self.cores.len() > self.core_count
    }

    /// Returns the core of one processor.
    ///
    /// `None` means `processor` lies outside this topology snapshot; table
    /// presence itself was proven when this view was constructed.
    #[must_use]
    pub fn core_of(self, processor: u32) -> Option<CoreId> {
        self.cores.get(usize::try_from(processor).ok()?).copied()
    }

    /// Returns whether two processors share an execution core.
    ///
    /// `None` when either lies outside the snapshot.
    #[must_use]
    pub fn are_siblings(self, a: u32, b: u32) -> Option<bool> {
        Some(self.core_of(a)? == self.core_of(b)?)
    }

    /// Iterates the processors of `core` by ascending logical id.
    #[must_use = "iterators are lazy; consume the returned processor iterator"]
    pub fn processors_in_core(self, core: CoreId) -> impl Iterator<Item = u32> + 'topology {
        self.cores
            .iter()
            .enumerate()
            .filter(move |(_, candidate)| **candidate == core)
            // Every construction path caps the table below `u32::MAX`.
            .filter_map(|(processor, _)| u32::try_from(processor).ok())
    }

    /// Iterates the other processors sharing the core of `processor`.
    ///
    /// `None` means `processor` lies outside the snapshot; a processor whose
    /// core has no other thread yields an empty iterator.
    #[must_use = "iterators are lazy; consume the returned processor iterator"]
    pub fn siblings_of(self, processor: u32) -> Option<impl Iterator<Item = u32> + 'topology> {
        let core = self.core_of(processor)?;
        Some(
            self.processors_in_core(core)
                .filter(move |candidate| *candidate != processor),
        )
    }

    /// Iterates one processor per core, the lowest of each, by ascending id.
    ///
    /// This is the set a measurement instrument pins to when it must not put
    /// two workers on one execution core, and the set a compute-bound pool
    /// sizes itself by.
    #[must_use = "iterators are lazy; consume the returned processor iterator"]
    pub fn one_processor_per_core(self) -> impl Iterator<Item = u32> + 'topology {
        // Dense ids are assigned by first appearance, so the first processor
        // of core `k` is the first whose id equals the count of cores seen.
        self.cores
            .iter()
            .enumerate()
            .scan(0u32, |seen, (processor, core)| {
                Some(if core.get() == *seen {
                    *seen += 1;
                    u32::try_from(processor).ok()
                } else {
                    None
                })
            })
            .flatten()
    }

    /// Builds group-partitioned native affinity masks for one processor per
    /// core.
    #[cfg(windows)]
    #[must_use]
    pub fn one_processor_per_core_affinity_groups(self) -> ProcessorAffinityGroups {
        ProcessorAffinityGroups::from_processors(self.one_processor_per_core())
    }
}

impl CpuTopology {
    /// Returns a presence-proven view of the reported SMT sibling partition.
    ///
    /// `None` preserves platform absence. Once this returns `Some`, the core
    /// count, SMT presence, and per-core processor sets are total operations on
    /// the borrowed snapshot. A host without SMT reports `Some` with one
    /// processor per core; absence means the platform said nothing usable.
    #[must_use]
    pub fn smt(&self) -> Option<CpuSmtView<'_>> {
        CpuSmtView::new(self.core_ids.as_deref()?)
    }
}
