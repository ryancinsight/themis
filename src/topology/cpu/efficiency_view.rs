//! Borrowed, presence-proven CPU efficiency-class queries.

use super::CpuTopology;
#[cfg(windows)]
use super::ProcessorAffinityGroups;
use crate::topology::types::EfficiencyClass;
use core::fmt;

/// Placement tier of an efficiency class within one topology snapshot.
///
/// Ranks are dense within a snapshot ([`EfficiencyClass`]), so the tier is the
/// only name a rank has: the highest rank is [`Performance`], rank `0` is
/// [`Efficiency`], every rank between them is [`Intermediate`], and a
/// snapshot with one class is [`Uniform`] because it has no tier to rank
/// against. Obtain a tier through [`CpuEfficiencyView::tier`].
///
/// [`Performance`]: Self::Performance
/// [`Efficiency`]: Self::Efficiency
/// [`Intermediate`]: Self::Intermediate
/// [`Uniform`]: Self::Uniform
///
/// # Examples
///
/// ```
/// # extern crate themis;
/// use themis::EfficiencyTier;
///
/// assert_eq!(EfficiencyTier::Intermediate.to_string(), "intermediate");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EfficiencyTier {
    /// The most performant class of a snapshot that reports several.
    Performance,
    /// The least performant class of a snapshot that reports several.
    Efficiency,
    /// A class strictly between the two, on a part with three or more tiers.
    Intermediate,
    /// The only class of a homogeneous snapshot.
    Uniform,
}

impl fmt::Display for EfficiencyTier {
    /// Writes the lowercase tier name: `performance`, `efficiency`,
    /// `intermediate` or `uniform`, honouring width and alignment.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.pad(match self {
            Self::Performance => "performance",
            Self::Efficiency => "efficiency",
            Self::Intermediate => "intermediate",
            Self::Uniform => "uniform",
        })
    }
}

/// A borrowed CPU efficiency-class table whose presence is already proven.
///
/// Construct this view through [`CpuTopology::efficiency`]. Class count,
/// hybrid status, the highest class and class iteration are total because the
/// optional platform report has been discharged once at the boundary. Lookups
/// by a caller-supplied processor index or class, such as [`Self::tier`],
/// remain optional because [`EfficiencyClass::new`] and an index can name
/// either outside the snapshot.
#[derive(Clone, Copy, Debug)]
pub struct CpuEfficiencyView<'topology> {
    classes: &'topology [EfficiencyClass],
    highest: EfficiencyClass,
}

impl<'topology> CpuEfficiencyView<'topology> {
    pub(super) fn new(classes: &'topology [EfficiencyClass]) -> Option<Self> {
        let highest = classes.iter().max().copied()?;
        Some(Self { classes, highest })
    }

    /// Returns the per-processor dense efficiency ranks.
    #[must_use]
    pub const fn classes(self) -> &'topology [EfficiencyClass] {
        self.classes
    }

    /// Returns how many distinct efficiency classes are represented.
    #[must_use]
    pub fn class_count(self) -> usize {
        usize::from(self.highest.rank()) + 1
    }

    /// Returns whether more than one efficiency class is represented.
    #[must_use]
    pub fn is_hybrid(self) -> bool {
        self.class_count() > 1
    }

    /// Returns the most performant represented class.
    #[must_use]
    pub const fn highest_class(self) -> EfficiencyClass {
        self.highest
    }

    /// Returns the placement tier of `class` in this snapshot.
    ///
    /// `None` means `class` ranks above the highest represented class, so this
    /// snapshot has no processor in it. Ranks are dense, so every class at or
    /// below the highest one is represented.
    #[must_use]
    pub fn tier(self, class: EfficiencyClass) -> Option<EfficiencyTier> {
        if class > self.highest {
            return None;
        }
        Some(if !self.is_hybrid() {
            EfficiencyTier::Uniform
        } else if class == self.highest {
            EfficiencyTier::Performance
        } else if class == EfficiencyClass::LOWEST {
            EfficiencyTier::Efficiency
        } else {
            EfficiencyTier::Intermediate
        })
    }

    /// Returns the placement tier of one processor.
    ///
    /// `None` means `processor` lies outside this topology snapshot.
    #[must_use]
    pub fn processor_tier(self, processor: u32) -> Option<EfficiencyTier> {
        self.tier(self.processor_class(processor)?)
    }

    /// Returns the represented class of one processor.
    ///
    /// `None` means `processor` lies outside this topology snapshot; class
    /// presence itself was proven when this view was constructed.
    #[must_use]
    pub fn processor_class(self, processor: u32) -> Option<EfficiencyClass> {
        self.classes.get(usize::try_from(processor).ok()?).copied()
    }

    /// Returns whether one processor belongs to the highest represented class.
    ///
    /// `None` means `processor` lies outside this topology snapshot.
    #[must_use]
    pub fn is_in_highest_class(self, processor: u32) -> Option<bool> {
        Some(self.processor_class(processor)? == self.highest)
    }

    /// Iterates processors in `class` by ascending logical id.
    #[must_use = "iterators are lazy; consume the returned processor iterator"]
    pub fn processors_in_class(
        self,
        class: EfficiencyClass,
    ) -> impl Iterator<Item = u32> + 'topology {
        self.classes
            .iter()
            .enumerate()
            .filter(move |(_, candidate)| **candidate == class)
            // Every construction path caps the table below `u32::MAX`.
            .filter_map(|(processor, _)| u32::try_from(processor).ok())
    }

    /// Iterates processors in the highest represented class.
    #[must_use = "iterators are lazy; consume the returned processor iterator"]
    pub fn highest_class_processors(self) -> impl Iterator<Item = u32> + 'topology {
        self.processors_in_class(self.highest)
    }

    /// Builds group-partitioned native affinity masks for one class.
    #[cfg(windows)]
    #[must_use]
    pub fn processor_affinity_groups(self, class: EfficiencyClass) -> ProcessorAffinityGroups {
        ProcessorAffinityGroups::from_processors(self.processors_in_class(class))
    }

    /// Builds group-partitioned native masks for the highest represented class.
    #[cfg(windows)]
    #[must_use]
    pub fn highest_class_affinity_groups(self) -> ProcessorAffinityGroups {
        self.processor_affinity_groups(self.highest)
    }
}

impl CpuTopology {
    /// Returns a presence-proven view of reported CPU efficiency classes.
    ///
    /// `None` preserves platform absence. Once this returns `Some`, class count,
    /// hybrid status, the highest class, and its processors are total
    /// operations on the borrowed snapshot. Windows additionally exposes
    /// group-aware native affinity masks through the returned view.
    #[must_use]
    pub fn efficiency(&self) -> Option<CpuEfficiencyView<'_>> {
        CpuEfficiencyView::new(self.efficiency_classes.as_deref()?)
    }
}
