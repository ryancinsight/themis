//! CPU and memory topology snapshots.

use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// NUMA node topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumaNode {
    /// Node identifier.
    pub id: NumaNodeId,
    /// Logical processors assigned to this node.
    pub processors: Box<[u32]>,
    /// Relative distance to other nodes.
    pub distances: Box<[u32]>,
    /// Primary memory tier for the node.
    pub memory_tier: MemoryTier,
}

/// Cache hierarchy level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLevel {
    /// Cache level.
    pub level: u32,
    /// Cache size in bytes.
    pub size_bytes: usize,
    /// Processors sharing this cache.
    pub shared_processors: Box<[u32]>,
}

/// CPU topology snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuTopology {
    /// Snapshot epoch.
    epoch: TopologyEpoch,
    numa_nodes: Box<[NumaNode]>,
    processor_to_node: Box<[Option<NumaNodeId>]>,
    node_to_index: Box<[Option<usize>]>,
    adjacent_nodes: Box<[Box<[NumaNodeId]>]>,
    logical_processors: usize,
    cache_levels: Box<[CacheLevel]>,
}

impl CpuTopology {
    /// Detects the CPU topology from the platform.
    #[must_use]
    pub fn detect() -> Option<Self> {
        #[cfg(all(feature = "std", target_os = "linux"))]
        {
            Self::detect_linux()
        }

        #[cfg(all(feature = "std", windows))]
        {
            Self::detect_windows()
        }

        #[cfg(not(any(
            all(feature = "std", target_os = "linux"),
            all(feature = "std", windows)
        )))]
        {
            Some(Self::single_node(logical_processor_count()))
        }
    }

    /// Creates a single-node topology.
    #[must_use]
    pub fn single_node(logical_processors: usize) -> Self {
        let logical_processors = logical_processors.max(1);
        let processors: Vec<u32> = (0..logical_processors as u32).collect();
        let node_id = NumaNodeId::ZERO;
        let processor_node_pairs: Vec<(u32, NumaNodeId)> = processors
            .iter()
            .map(|processor| (*processor, node_id))
            .collect();
        let numa_nodes = vec![NumaNode {
            id: node_id,
            processors: processors.into_boxed_slice(),
            distances: vec![10].into_boxed_slice(),
            memory_tier: MemoryTier::Dram,
        }];

        Self {
            epoch: TopologyEpoch::INITIAL,
            processor_to_node: build_processor_to_node(logical_processors, &processor_node_pairs),
            node_to_index: build_node_to_index(&numa_nodes),
            adjacent_nodes: build_adjacent_nodes(&numa_nodes),
            numa_nodes: numa_nodes.into_boxed_slice(),
            logical_processors,
            cache_levels: default_cache_levels(logical_processors),
        }
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

    /// Returns the cache hierarchy table.
    #[must_use]
    pub fn cache_levels(&self) -> &[CacheLevel] {
        &self.cache_levels
    }

    /// Returns the logical processor count.
    #[must_use]
    pub const fn logical_processors(&self) -> usize {
        self.logical_processors
    }

    /// Returns the NUMA node for a processor.
    #[must_use]
    pub fn processor_to_numa_node(&self, processor: u32) -> Option<NumaNodeId> {
        self.processor_to_node
            .get(processor as usize)
            .copied()
            .flatten()
    }

    /// Iterates over known processor-to-node mappings.
    pub fn processor_node_pairs(&self) -> impl Iterator<Item = (u32, NumaNodeId)> + '_ {
        self.processor_to_node
            .iter()
            .enumerate()
            .filter_map(|(processor, node)| Some((processor as u32, (*node)?)))
    }

    /// Returns node distance.
    #[must_use]
    pub fn distance(&self, from: NumaNodeId, to: NumaNodeId) -> u32 {
        match (self.node_index(from), self.node_index(to)) {
            (Some(from_index), Some(to_index)) => self
                .numa_nodes
                .get(from_index)
                .and_then(|node| node.distances.get(to_index).copied())
                .unwrap_or(if from == to { 10 } else { 20 }),
            _ => {
                if from == to {
                    10
                } else {
                    20
                }
            }
        }
    }

    /// Returns the compact topology index for a NUMA node ID.
    #[must_use]
    pub fn node_index(&self, node_id: NumaNodeId) -> Option<usize> {
        self.node_to_index.get(node_id.index()).copied().flatten()
    }

    /// Returns adjacent nodes sorted by distance.
    #[must_use]
    pub fn adjacent_nodes(&self, node_id: NumaNodeId) -> &[NumaNodeId] {
        self.node_index(node_id)
            .and_then(|index| self.adjacent_nodes.get(index))
            .map_or(&[], |nodes| nodes)
    }

    #[cfg(all(feature = "std", target_os = "linux"))]
    fn detect_linux() -> Option<Self> {
        use std::fs;

        let nodes_path = "/sys/devices/system/node/";
        let node_entries = fs::read_dir(nodes_path).ok();
        let mut node_ids: Vec<u32> = node_entries
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter_map(|name| name.strip_prefix("node")?.parse::<u32>().ok())
            .collect();
        node_ids.sort_unstable();

        if node_ids.is_empty() {
            return Some(Self::single_node(logical_processor_count()));
        }

        let mut numa_nodes = Vec::with_capacity(node_ids.len());
        let mut processor_node_pairs = Vec::new();

        for node_id_raw in &node_ids {
            let node_id = NumaNodeId::new(*node_id_raw);
            let cpulist_path = format!("{nodes_path}/node{node_id_raw}/cpulist");
            let processors = fs::read_to_string(cpulist_path)
                .map(|value| parse_cpu_list(&value))
                .unwrap_or_default();

            for processor in &processors {
                processor_node_pairs.push((*processor, node_id));
            }

            let distance_path = format!("{nodes_path}/node{node_id_raw}/distance");
            let distances = fs::read_to_string(distance_path)
                .map(|value| {
                    value
                        .split_whitespace()
                        .filter_map(|part| part.parse::<u32>().ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|_| vec![10; node_ids.len()]);

            numa_nodes.push(NumaNode {
                id: node_id,
                processors: processors.into_boxed_slice(),
                distances: distances.into_boxed_slice(),
                memory_tier: MemoryTier::Dram,
            });
        }

        let logical_processors = logical_processor_count();
        let processor_to_node = build_processor_to_node(logical_processors, &processor_node_pairs);
        let node_to_index = build_node_to_index(&numa_nodes);
        let adjacent_nodes = build_adjacent_nodes(&numa_nodes);
        Some(Self {
            epoch: TopologyEpoch::INITIAL,
            numa_nodes: numa_nodes.into_boxed_slice(),
            node_to_index,
            processor_to_node,
            adjacent_nodes,
            logical_processors,
            cache_levels: default_cache_levels(logical_processors),
        })
    }

    #[cfg(all(feature = "std", windows))]
    fn detect_windows() -> Option<Self> {
        extern "system" {
            fn GetNumaHighestNodeNumber(highest_node_number: *mut u32) -> i32;
            fn GetNumaNodeProcessorMask(node: u8, processor_mask: *mut u64) -> i32;
        }

        let mut highest_node = 0u32;
        // SAFETY: The API writes one `u32` through a valid output pointer.
        if unsafe { GetNumaHighestNodeNumber(&mut highest_node) } == 0 {
            return Some(Self::single_node(logical_processor_count()));
        }

        let node_count = highest_node.saturating_add(1) as usize;
        let mut numa_nodes = Vec::with_capacity(node_count);
        let mut processor_node_pairs = Vec::new();
        let mut logical_processors = 0usize;

        for raw_node in 0..=highest_node {
            let mut mask = 0u64;
            // SAFETY: The API writes one processor mask through a valid pointer.
            if unsafe { GetNumaNodeProcessorMask(raw_node as u8, &mut mask) } == 0 || mask == 0 {
                continue;
            }
            let node_id = NumaNodeId::new(raw_node);
            let mut processors = Vec::new();
            for processor in 0..64u32 {
                if (mask & (1u64 << processor)) != 0 {
                    processors.push(processor);
                    processor_node_pairs.push((processor, node_id));
                    logical_processors = logical_processors.max(processor as usize + 1);
                }
            }
            numa_nodes.push(NumaNode {
                id: node_id,
                processors: processors.into_boxed_slice(),
                distances: (0..node_count)
                    .map(|index| if index == raw_node as usize { 10 } else { 20 })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                memory_tier: MemoryTier::Dram,
            });
        }

        if numa_nodes.is_empty() {
            return Some(Self::single_node(logical_processor_count()));
        }

        Some(Self {
            epoch: TopologyEpoch::INITIAL,
            node_to_index: build_node_to_index(&numa_nodes),
            adjacent_nodes: build_adjacent_nodes(&numa_nodes),
            numa_nodes: numa_nodes.into_boxed_slice(),
            processor_to_node: build_processor_to_node(
                logical_processors.max(1),
                &processor_node_pairs,
            ),
            logical_processors: logical_processors.max(1),
            cache_levels: default_cache_levels(logical_processors.max(1)),
        })
    }
}

fn build_processor_to_node(
    logical_processors: usize,
    mappings: &[(u32, NumaNodeId)],
) -> Box<[Option<NumaNodeId>]> {
    let max_processor = mappings
        .iter()
        .map(|(processor, _)| *processor as usize)
        .max()
        .unwrap_or(0);
    let mut processor_to_node = vec![None; logical_processors.max(max_processor + 1).max(1)];
    for (processor, node) in mappings {
        processor_to_node[*processor as usize] = Some(*node);
    }
    processor_to_node.into_boxed_slice()
}

fn build_node_to_index(nodes: &[NumaNode]) -> Box<[Option<usize>]> {
    let max_node = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let mut node_to_index = vec![None; max_node + 1];
    for (index, node) in nodes.iter().enumerate() {
        node_to_index[node.id.index()] = Some(index);
    }
    node_to_index.into_boxed_slice()
}

fn build_adjacent_nodes(nodes: &[NumaNode]) -> Box<[Box<[NumaNodeId]>]> {
    nodes
        .iter()
        .enumerate()
        .map(|(from_index, from_node)| {
            let mut adjacent: Vec<(NumaNodeId, u32)> = nodes
                .iter()
                .enumerate()
                .filter(|(to_index, _)| *to_index != from_index)
                .map(|(to_index, to_node)| {
                    let distance = from_node
                        .distances
                        .get(to_index)
                        .copied()
                        .unwrap_or(if from_node.id == to_node.id { 10 } else { 20 });
                    (to_node.id, distance)
                })
                .collect();
            adjacent.sort_by_key(|(_, distance)| *distance);
            adjacent
                .into_iter()
                .map(|(node_id, _)| node_id)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[cfg(all(feature = "std", target_os = "linux"))]
fn parse_cpu_list(cpulist: &str) -> Vec<u32> {
    let mut processors = Vec::new();
    for part in cpulist.trim().split(',') {
        if let Some((start, end)) = part.split_once('-') {
            if let (Ok(start), Ok(end)) = (start.parse::<u32>(), end.parse::<u32>()) {
                processors.extend(start..=end);
            }
        } else if let Ok(processor) = part.parse::<u32>() {
            processors.push(processor);
        }
    }
    processors
}

fn default_cache_levels(logical_processors: usize) -> Box<[CacheLevel]> {
    let processors: Vec<u32> = (0..logical_processors.max(1) as u32).collect();
    vec![
        CacheLevel {
            level: 1,
            size_bytes: 32 * 1024,
            shared_processors: Box::default(),
        },
        CacheLevel {
            level: 2,
            size_bytes: 256 * 1024,
            shared_processors: Box::default(),
        },
        CacheLevel {
            level: 3,
            size_bytes: 8 * 1024 * 1024,
            shared_processors: processors.into_boxed_slice(),
        },
    ]
    .into_boxed_slice()
}

fn logical_processor_count() -> usize {
    #[cfg(feature = "std")]
    {
        std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
    }

    #[cfg(not(feature = "std"))]
    {
        1
    }
}

/// GPU device topology snapshot (atlas ADR 0002).
///
/// Provider-fed: themis stays stateless law, so there is no `detect()` here —
/// device backends (hephaestus) construct this from wgpu adapter limits or
/// CUDA device attributes via [`GpuTopology::from_provider`]. Consumers:
/// moirai's occupancy planner (warp-aware launch shaping) and mnemosyne's
/// kernel resource budgets read these capacities; the `Registers`/`SharedMem`
/// figures are budget vocabulary, never host-allocatable (see
/// [`MemoryTier::is_host_allocatable`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuTopology {
    epoch: TopologyEpoch,
    properties: GpuDeviceProperties,
}

/// Provider-supplied GPU device properties for [`GpuTopology::from_provider`].
///
/// A plain field struct (not a builder): every field is required, and the
/// provider reads them directly off the device API in one place. Fields the
/// API does not report are zero (capacity unknown), never fabricated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuDeviceProperties {
    /// Streaming-multiprocessor / compute-unit count (0 when unreported).
    pub compute_units: u32,
    /// Warp (NVIDIA) / wavefront (AMD) / subgroup width in lanes.
    pub warp_width: u32,
    /// Maximum resident threads per compute unit (0 when unreported).
    pub max_threads_per_unit: u32,
    /// 32-bit registers per compute unit (budgeted tier `Registers`;
    /// 0 when unreported).
    pub registers_per_unit: u32,
    /// Shared/local memory bytes per compute unit (budgeted tier
    /// `SharedMem`).
    pub shared_mem_per_unit_bytes: usize,
    /// Device L2 cache size in bytes (0 when unreported).
    pub l2_bytes: usize,
    /// Device global-memory tier (`Hbm`, `Gddr`, or `Device` when unknown).
    pub memory_tier: MemoryTier,
    /// Device global-memory capacity in bytes (0 when unreported).
    pub memory_bytes: u64,
}

impl GpuTopology {
    /// Construct a snapshot from provider-reported device properties.
    #[must_use]
    pub const fn from_provider(properties: GpuDeviceProperties) -> Self {
        Self {
            epoch: TopologyEpoch::INITIAL,
            properties,
        }
    }

    /// Snapshot epoch.
    #[must_use]
    #[inline]
    pub const fn epoch(&self) -> TopologyEpoch {
        self.epoch
    }

    /// Streaming-multiprocessor / compute-unit count.
    #[must_use]
    #[inline]
    pub const fn compute_units(&self) -> u32 {
        self.properties.compute_units
    }

    /// Warp / wavefront / subgroup width in lanes.
    #[must_use]
    #[inline]
    pub const fn warp_width(&self) -> u32 {
        self.properties.warp_width
    }

    /// Maximum resident threads per compute unit.
    #[must_use]
    #[inline]
    pub const fn max_threads_per_unit(&self) -> u32 {
        self.properties.max_threads_per_unit
    }

    /// 32-bit registers per compute unit (budgeted `Registers` tier).
    #[must_use]
    #[inline]
    pub const fn registers_per_unit(&self) -> u32 {
        self.properties.registers_per_unit
    }

    /// Shared/local memory bytes per compute unit (budgeted `SharedMem` tier).
    #[must_use]
    #[inline]
    pub const fn shared_mem_per_unit_bytes(&self) -> usize {
        self.properties.shared_mem_per_unit_bytes
    }

    /// Device L2 cache size in bytes (0 when unreported).
    #[must_use]
    #[inline]
    pub const fn l2_bytes(&self) -> usize {
        self.properties.l2_bytes
    }

    /// Device global-memory tier.
    #[must_use]
    #[inline]
    pub const fn memory_tier(&self) -> MemoryTier {
        self.properties.memory_tier
    }

    /// Device global-memory capacity in bytes.
    #[must_use]
    #[inline]
    pub const fn memory_bytes(&self) -> u64 {
        self.properties.memory_bytes
    }

    /// Total resident warps at theoretical full occupancy:
    /// `compute_units · max_threads_per_unit / warp_width`. Returns 0 for a
    /// zero warp width rather than dividing by zero.
    #[must_use]
    #[inline]
    pub const fn max_resident_warps(&self) -> u64 {
        if self.properties.warp_width == 0 {
            return 0;
        }
        (self.properties.compute_units as u64) * (self.properties.max_threads_per_unit as u64)
            / (self.properties.warp_width as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_node_maps_every_processor_to_node_zero() {
        let topology = CpuTopology::single_node(4);
        assert_eq!(topology.numa_nodes.len(), 1);
        assert_eq!(topology.logical_processors, 4);
        for processor in 0..4 {
            assert_eq!(
                topology.processor_to_numa_node(processor),
                Some(NumaNodeId::ZERO)
            );
        }
    }

    #[test]
    fn distance_defaults_preserve_self_and_remote_values() {
        let topology = CpuTopology::single_node(1);
        assert_eq!(topology.distance(NumaNodeId::ZERO, NumaNodeId::ZERO), 10);
        assert_eq!(topology.distance(NumaNodeId::ZERO, NumaNodeId::new(9)), 20);
    }

    #[test]
    fn detected_topology_has_at_least_one_node() {
        let topology = CpuTopology::detect().expect("topology detection should return fallback");
        assert!(!topology.numa_nodes.is_empty());
        assert!(topology.logical_processors > 0);
    }

    #[test]
    fn sparse_node_ids_use_compact_distance_rows() {
        let nodes = vec![
            NumaNode {
                id: NumaNodeId::new(2),
                processors: vec![0].into_boxed_slice(),
                distances: vec![10, 31].into_boxed_slice(),
                memory_tier: MemoryTier::Dram,
            },
            NumaNode {
                id: NumaNodeId::new(7),
                processors: vec![1].into_boxed_slice(),
                distances: vec![31, 10].into_boxed_slice(),
                memory_tier: MemoryTier::Dram,
            },
        ];
        let topology = CpuTopology {
            epoch: TopologyEpoch::INITIAL,
            processor_to_node: build_processor_to_node(
                2,
                &[(0, NumaNodeId::new(2)), (1, NumaNodeId::new(7))],
            ),
            node_to_index: build_node_to_index(&nodes),
            adjacent_nodes: build_adjacent_nodes(&nodes),
            numa_nodes: nodes.into_boxed_slice(),
            logical_processors: 2,
            cache_levels: default_cache_levels(2),
        };

        assert_eq!(topology.processor_to_numa_node(1), Some(NumaNodeId::new(7)));
        assert_eq!(
            topology.distance(NumaNodeId::new(2), NumaNodeId::new(7)),
            31
        );
        assert_eq!(topology.node_index(NumaNodeId::new(7)), Some(1));
        assert_eq!(
            topology.adjacent_nodes(NumaNodeId::new(2)),
            &[NumaNodeId::new(7)]
        );
    }
}

#[cfg(test)]
mod gpu_topology_tests {
    use super::*;

    fn sample_properties() -> GpuDeviceProperties {
        GpuDeviceProperties {
            compute_units: 46,
            warp_width: 32,
            max_threads_per_unit: 1536,
            registers_per_unit: 65536,
            shared_mem_per_unit_bytes: 102_400,
            l2_bytes: 4 * 1024 * 1024,
            memory_tier: MemoryTier::Gddr,
            memory_bytes: 8 * 1024 * 1024 * 1024,
        }
    }

    #[test]
    fn provider_snapshot_round_trips_every_field() {
        let topology = GpuTopology::from_provider(sample_properties());
        assert_eq!(topology.compute_units(), 46);
        assert_eq!(topology.warp_width(), 32);
        assert_eq!(topology.max_threads_per_unit(), 1536);
        assert_eq!(topology.registers_per_unit(), 65536);
        assert_eq!(topology.shared_mem_per_unit_bytes(), 102_400);
        assert_eq!(topology.l2_bytes(), 4 * 1024 * 1024);
        assert_eq!(topology.memory_tier(), MemoryTier::Gddr);
        assert_eq!(topology.memory_bytes(), 8 * 1024 * 1024 * 1024);
        assert_eq!(topology.epoch(), TopologyEpoch::INITIAL);
    }

    #[test]
    fn max_resident_warps_is_units_times_threads_over_width() {
        let topology = GpuTopology::from_provider(sample_properties());
        // 46 * 1536 / 32 = 2208
        assert_eq!(topology.max_resident_warps(), 2208);

        let mut zero_width = sample_properties();
        zero_width.warp_width = 0;
        assert_eq!(
            GpuTopology::from_provider(zero_width).max_resident_warps(),
            0
        );
    }

    #[test]
    fn budgeted_tiers_are_not_host_allocatable() {
        assert!(!MemoryTier::Registers.is_host_allocatable());
        assert!(!MemoryTier::SharedMem.is_host_allocatable());
        assert!(MemoryTier::Gddr.is_host_allocatable());
        assert!(MemoryTier::HostPinned.is_host_allocatable());
        assert!(MemoryTier::Hbm.is_host_allocatable());
        assert!(MemoryTier::Dram.is_host_allocatable());
    }
}
