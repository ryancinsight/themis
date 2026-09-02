//! The `CpuTopology` snapshot: construction paths, accessors, and the
//! detection entry point that assembles one from the platform backends.

use super::super::types::{CacheLevel, CoreId, EfficiencyClass, NumaNode};
use super::{build_adjacent_nodes, build_node_to_index, tables, LOCAL_DISTANCE, REMOTE_DISTANCE};
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

/// CPU topology snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Snapshot epoch.
    pub(crate) epoch: TopologyEpoch,
    pub(crate) numa_nodes: Box<[NumaNode]>,
    pub(crate) processor_to_node: Box<[NumaNodeId]>,
    pub(crate) node_to_index: Box<[usize]>,
    pub(crate) adjacent_nodes: Box<[NumaNodeId]>,
    pub(crate) logical_processors: usize,
    pub(crate) cache_levels: Option<Box<[CacheLevel]>>,
    /// Dense efficiency rank per logical processor, indexed by processor id.
    ///
    /// Invariant on `Some`: the table has exactly `logical_processors` entries
    /// and its ranks are the contiguous range `0..distinct_count`. Every
    /// construction path either satisfies both or stores `None`.
    pub(crate) efficiency_classes: Option<Box<[EfficiencyClass]>>,
    /// Physical core per logical processor, indexed by processor id.
    ///
    /// Invariant on `Some`: the table has exactly `logical_processors` entries
    /// and its ids are the contiguous range `0..core_count`, assigned by
    /// ascending lowest processor. Every construction path either satisfies
    /// both or stores `None`.
    pub(crate) core_ids: Option<Box<[CoreId]>>,
}

impl CpuTopology {
    /// Creates a single-node topology.
    ///
    /// # Panics
    ///
    /// Panics if `logical_processors` exceeds `u32::MAX`; processor ids are
    /// `u32` throughout this crate, so a wider count has no representation.
    #[must_use]
    pub fn single_node(logical_processors: usize) -> Self {
        let logical_processors = logical_processors.max(1);
        let processor_count = u32::try_from(logical_processors)
            .expect("invariant: logical processor count must fit a u32 processor id");
        let processors: Box<[u32]> = (0..processor_count).collect();
        let node_id = NumaNodeId::ZERO;
        let numa_nodes: Box<[NumaNode]> = Box::new([NumaNode {
            id: node_id,
            processors,
            distances: Box::new([LOCAL_DISTANCE]),
            memory_tier: MemoryTier::Dram,
        }]);

        Self {
            epoch: TopologyEpoch::INITIAL,
            processor_to_node: vec![node_id; logical_processors].into_boxed_slice(),
            node_to_index: build_node_to_index(&numa_nodes),
            adjacent_nodes: build_adjacent_nodes(&numa_nodes),
            numa_nodes,
            logical_processors,
            cache_levels: None,
            efficiency_classes: None,
            core_ids: None,
        }
    }

    /// Construct a topology from primary fields for testing.
    ///
    /// `node_to_index` and `adjacent_nodes` are derived from `numa_nodes`.
    ///
    /// Gated on `feature = "testing"` (not just `cfg(test)`) because integration
    /// tests in `tests/` consume the lib as a regular dependency; `cfg(test)` only
    /// activates when the lib itself is the test target, not when it is depended on.
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn new_for_test(
        epoch: TopologyEpoch,
        numa_nodes: Box<[NumaNode]>,
        processor_to_node: Box<[NumaNodeId]>,
        logical_processors: usize,
        cache_levels: Option<Box<[CacheLevel]>>,
    ) -> Self {
        let node_to_index = build_node_to_index(&numa_nodes);
        let adjacent_nodes = build_adjacent_nodes(&numa_nodes);
        Self {
            epoch,
            numa_nodes,
            processor_to_node,
            node_to_index,
            adjacent_nodes,
            logical_processors,
            cache_levels,
            efficiency_classes: None,
            core_ids: None,
        }
    }

    /// Attaches an efficiency-class table to a test topology.
    ///
    /// Additive companion to [`Self::new_for_test`], whose signature is a
    /// published contract. Panics are the test contract here: a table that
    /// violates the field invariant would make the accessors meaningless.
    ///
    /// # Panics
    ///
    /// Panics if the table's length is not `logical_processors`, or if its
    /// ranks are not the contiguous range `0..distinct_count`.
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn with_efficiency_classes_for_test(
        mut self,
        efficiency_classes: Option<Box<[EfficiencyClass]>>,
    ) -> Self {
        if let Some(classes) = efficiency_classes.as_deref() {
            assert_eq!(
                classes.len(),
                self.logical_processors,
                "invariant: the class table covers every logical processor"
            );
            // Density without allocating, so the constructor stays available
            // wherever `new_for_test` is, `no_std` included.
            let mut present = [false; 256];
            for class in classes {
                present[usize::from(class.rank())] = true;
            }
            if let Some(highest) = classes.iter().max() {
                assert!(
                    present
                        .iter()
                        .take(usize::from(highest.rank()) + 1)
                        .all(|seen| *seen),
                    "invariant: efficiency ranks are dense"
                );
            }
        }
        self.efficiency_classes = efficiency_classes;
        self
    }

    /// Attaches a core table to a test topology.
    ///
    /// Additive companion to [`Self::new_for_test`]. Panics are the test
    /// contract here: a table that violates the field invariant would make the
    /// SMT view meaningless.
    ///
    /// # Panics
    ///
    /// Panics if the table length is not `logical_processors`, or if its ids
    /// are not assigned densely by first appearance.
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn with_core_ids_for_test(mut self, core_ids: Option<Box<[CoreId]>>) -> Self {
        if let Some(cores) = core_ids.as_deref() {
            assert_eq!(
                cores.len(),
                self.logical_processors,
                "invariant: the core table covers every logical processor"
            );
            let mut next = 0u32;
            for core in cores {
                assert!(
                    core.get() <= next,
                    "invariant: core ids are dense and assigned by first appearance"
                );
                if core.get() == next {
                    next += 1;
                }
            }
        }
        self.core_ids = core_ids;
        self
    }

    /// Returns the snapshot epoch.
    #[must_use]
    pub const fn epoch(&self) -> TopologyEpoch {
        self.epoch
    }

    /// Returns the NUMA node table.
    #[must_use]
    pub fn numa_nodes(&self) -> &[NumaNode] {
        &self.numa_nodes
    }

    /// Returns the platform-reported cache hierarchy table.
    ///
    /// # Provenance
    ///
    /// `None` means the platform did not report a complete cache hierarchy.
    /// The single-node constructor never fabricates cache values. Linux reads
    /// cache-index records from sysfs, and Windows reads
    /// `GetLogicalProcessorInformationEx`; malformed or unavailable platform
    /// data remains typed absence. Consumers that tile on cache size must
    /// preserve that absence instead of substituting a machine-independent
    /// guess.
    #[must_use]
    pub fn cache_levels(&self) -> Option<&[CacheLevel]> {
        self.cache_levels.as_deref()
    }

    /// Returns the logical processor count.
    #[must_use]
    pub const fn logical_processors(&self) -> usize {
        self.logical_processors
    }

    /// Returns the platform-reported efficiency class of every processor,
    /// indexed by processor id.
    ///
    /// # Provenance
    ///
    /// `None` means the platform did not report a class for every logical
    /// processor. The single-node constructor never fabricates classes. Linux
    /// reads the Intel hybrid CPU-type `cpulist`s and then ARM `cpu_capacity`
    /// from sysfs; Windows reads the `EfficiencyClass` byte of each
    /// `GetLogicalProcessorInformationEx(RelationProcessorCore)` record; every
    /// other target reports absence. A host is never inferred to be hybrid from
    /// core counts, processor ids, or model strings.
    ///
    /// Consumers that pin threads by class must preserve that absence instead
    /// of substituting a machine-specific guess: on the host that motivated
    /// this accessor the performance cores are the non-contiguous mask
    /// `0xc03c03`, so "the low ids are the fast ones" selects one of the
    /// slowest processors on the part.
    ///
    /// `Some` with a single distinct class is a homogeneous host, which is the
    /// common case and is deliberately distinguishable from `None`.
    #[must_use]
    pub fn efficiency_classes(&self) -> Option<&[EfficiencyClass]> {
        self.efficiency_classes.as_deref()
    }

    /// Returns the reported physical core of every logical processor, indexed
    /// by processor id, or `None` when the platform reported nothing usable.
    ///
    /// Prefer [`Self::smt`], which discharges presence once and answers the
    /// sibling questions directly.
    #[must_use]
    pub fn core_ids(&self) -> Option<&[CoreId]> {
        self.core_ids.as_deref()
    }

    /// Returns the efficiency class of one processor.
    ///
    /// `None` when the platform reported no classes (see
    /// [`Self::efficiency_classes`]) or when the processor is outside this
    /// snapshot.
    #[must_use]
    pub fn processor_efficiency_class(&self, processor: u32) -> Option<EfficiencyClass> {
        self.efficiency()?.processor_class(processor)
    }

    /// Returns how many distinct efficiency classes the platform reported.
    ///
    /// This is the absence oracle for the rest of the efficiency surface:
    /// `None` is "the platform did not say", `Some(1)` is a homogeneous host,
    /// and `Some(n)` for `n > 1` is a hybrid host with `n` tiers.
    #[must_use]
    pub fn efficiency_class_count(&self) -> Option<usize> {
        Some(self.efficiency()?.class_count())
    }

    /// Returns whether the host mixes cores of different performance classes.
    ///
    /// `None` is typed absence, not "no". A consumer must not read an unasked
    /// question as a homogeneous answer.
    #[must_use]
    pub fn is_hybrid(&self) -> Option<bool> {
        Some(self.efficiency()?.is_hybrid())
    }

    /// Returns the most performant class the platform reported.
    ///
    /// On a homogeneous host this is [`EfficiencyClass::LOWEST`], the only
    /// class present; the count from [`Self::efficiency_class_count`] is what
    /// distinguishes that from a hybrid host's top tier.
    #[must_use]
    pub fn highest_efficiency_class(&self) -> Option<EfficiencyClass> {
        Some(self.efficiency()?.highest_class())
    }

    /// Returns whether `processor` sits in the most performant reported class.
    ///
    /// `None` is typed absence: either the platform reported no classes, or
    /// `processor` is outside this snapshot. `Some(false)` means the platform
    /// answered and this processor is not in the top tier.
    ///
    /// # Why this exists
    ///
    /// The natural spelling of this question compares the two accessors
    /// directly, and that spelling silently fabricates an answer:
    ///
    /// ```
    /// use themis::CpuTopology;
    ///
    /// // A host that reported no classes: both accessors are `None`.
    /// let unreported = CpuTopology::single_node(8);
    /// assert_eq!(unreported.processor_efficiency_class(0), None);
    /// assert_eq!(unreported.highest_efficiency_class(), None);
    ///
    /// // `None == None`, so the comparison claims every processor is a
    /// // performance core — including one that does not exist.
    /// assert!(
    ///     unreported.processor_efficiency_class(999)
    ///         == unreported.highest_efficiency_class()
    /// );
    ///
    /// // The predicate preserves the absence instead of inventing a yes.
    /// assert_eq!(unreported.is_in_highest_class(0), None);
    /// assert_eq!(unreported.is_in_highest_class(999), None);
    /// ```
    ///
    /// On a homogeneous host every reported processor is in the highest class,
    /// because there is only one; [`Self::efficiency_class_count`] is what
    /// distinguishes that from a hybrid host's top tier.
    #[must_use]
    pub fn is_in_highest_class(&self, processor: u32) -> Option<bool> {
        self.efficiency()?.is_in_highest_class(processor)
    }

    /// Iterates the processors of one efficiency class, in ascending id order.
    ///
    /// `None` is typed absence — the platform reported no classes — and must
    /// not be read as "this host has no processors of that class". A reported
    /// class below [`Self::efficiency_class_count`] always yields at least one
    /// processor, because ranks are dense; a class at or above it yields none.
    ///
    /// Pair with [`Self::highest_efficiency_class`] to select a representative
    /// performance processor without hardcoding an id.
    ///
    /// # Examples
    ///
    /// Selecting a processor to pin a latency-sensitive probe to, preserving
    /// absence rather than falling back to a machine-specific constant:
    ///
    /// ```
    /// use themis::CpuTopology;
    ///
    /// fn performance_processor(topology: &CpuTopology) -> Option<u32> {
    ///     let fastest = topology.highest_efficiency_class()?;
    ///     topology.processors_in_efficiency_class(fastest)?.next()
    /// }
    ///
    /// // A topology that reports no classes yields no processor, rather than
    /// // guessing one.
    /// let unreported = CpuTopology::single_node(8);
    /// assert_eq!(unreported.efficiency_classes(), None);
    /// assert_eq!(unreported.is_hybrid(), None);
    /// assert_eq!(performance_processor(&unreported), None);
    /// ```
    #[must_use = "iterators are lazy; consume the returned processor iterator"]
    pub fn processors_in_efficiency_class(
        &self,
        class: EfficiencyClass,
    ) -> Option<impl Iterator<Item = u32> + '_> {
        Some(self.efficiency()?.processors_in_class(class))
    }

    /// Returns the NUMA node for a processor.
    #[must_use]
    pub fn processor_to_numa_node(&self, processor: u32) -> Option<NumaNodeId> {
        self.processor_to_node
            .get(processor as usize)
            .copied()
            .filter(|&node_id| node_id != NumaNodeId::INVALID)
    }

    /// Iterates over known processor-to-node mappings.
    ///
    /// # Panics
    ///
    /// The returned iterator panics if the processor table is longer than
    /// `u32::MAX`, which construction already caps at 32768 entries.
    #[must_use = "iterators are lazy; consume the returned mapping iterator"]
    pub fn processor_node_pairs(&self) -> impl Iterator<Item = (u32, NumaNodeId)> + '_ {
        self.processor_to_node
            .iter()
            .enumerate()
            .filter(|(_, &node)| node != NumaNodeId::INVALID)
            .map(|(processor, &node)| {
                // The processor table is capped at 32768 entries when built.
                let processor = u32::try_from(processor)
                    .expect("invariant: processor table length is capped at 32768");
                (processor, node)
            })
    }

    /// Returns node distance (ACPI SLIT convention: `10` = local, higher =
    /// farther).
    ///
    /// # Provenance
    ///
    /// Only the **Linux** backend reads real inter-node distances (from
    /// `/sys/devices/system/node/nodeN/distance`), falling back to the
    /// synthetic `10`/`20` matrix on read failure. The **Windows** backend has
    /// no distance API without `GetLogicalProcessorInformationEx` relative-
    /// distance parsing, so it always returns the synthetic `10` (local) /
    /// `20` (remote) — uniform regardless of true inter-node latency. Consumers
    /// that weight placement by distance must treat a Windows result as a
    /// two-tier local/remote hint, not a measured latency.
    #[must_use]
    pub fn distance(&self, from: NumaNodeId, to: NumaNodeId) -> u32 {
        match (self.node_index(from), self.node_index(to)) {
            (Some(from_index), Some(to_index)) => self
                .numa_nodes
                .get(from_index)
                .and_then(|node| {
                    let max_node_id = self.node_to_index.len().saturating_sub(1);
                    let idx = if node.distances.len() > max_node_id {
                        to.index()
                    } else {
                        to_index
                    };
                    node.distances.get(idx).copied()
                })
                .unwrap_or_else(|| tables::default_distance(from_index, to_index)),
            _ => {
                if from == to {
                    LOCAL_DISTANCE
                } else {
                    REMOTE_DISTANCE
                }
            }
        }
    }

    /// Returns the compact topology index for a NUMA node ID.
    #[must_use]
    pub fn node_index(&self, node_id: NumaNodeId) -> Option<usize> {
        self.node_to_index
            .get(node_id.index())
            .copied()
            .filter(|&index| index != usize::MAX)
    }

    /// Returns adjacent nodes sorted by distance.
    #[must_use]
    pub fn adjacent_nodes(&self, node_id: NumaNodeId) -> &[NumaNodeId] {
        if let Some(index) = self.node_index(node_id) {
            let node_count = self.numa_nodes.len();
            if node_count <= 1 {
                return &[];
            }
            let stride = node_count - 1;
            let start = index * stride;
            let end = start + stride;
            self.adjacent_nodes.get(start..end).unwrap_or(&[])
        } else {
            &[]
        }
    }
}

pub(crate) fn logical_processor_count() -> usize {
    #[cfg(feature = "std")]
    {
        std::thread::available_parallelism().map_or(1, usize::from)
    }

    #[cfg(not(feature = "std"))]
    {
        1
    }
}
