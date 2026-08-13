//! Windows NUMA API CPU topology detection.

use super::super::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
    detect_cache_levels, logical_processor_count, CpuTopology,
};
use crate::law::{MemoryTier, NumaNodeId, TopologyEpoch};
use crate::topology::types::NumaNode;

// `CpuTopology::detect` exposes `Option<CpuTopology>` publicly: it models
// "no backend could produce a topology". Every current backend resolves to at
// least a single-node snapshot, but diverging one backend's signature would
// fork the seam that public contract sits on.
#[expect(
    clippy::unnecessary_wraps,
    reason = "platform detect backends share the Option-returning contract of CpuTopology::detect"
)]
pub(super) fn detect() -> Option<CpuTopology> {
    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    struct GroupAffinity {
        mask: usize,
        group: u16,
        reserved: [u16; 3],
    }

    // SAFETY: the declarations match the Win32 signatures for these entry
    // points (`GetNumaHighestNodeNumber`, `GetNumaNodeProcessorMaskEx`,
    // `systemtopologyapi.h`): out-pointer to `ULONG`, `USHORT` node index with
    // an out-pointer to `GROUP_AFFINITY`, `BOOL` results. `GroupAffinity`
    // above is `#[repr(C)]` with the same field order and widths.
    unsafe extern "system" {
        fn GetNumaHighestNodeNumber(highest_node_number: *mut u32) -> i32;
        fn GetNumaNodeProcessorMaskEx(node: u16, processor_mask: *mut GroupAffinity) -> i32;
    }

    let mut highest_node = 0u32;
    // SAFETY: The API writes one `u32` through a valid output pointer.
    if unsafe { GetNumaHighestNodeNumber(&mut highest_node) } == 0 || highest_node >= 1024 {
        return Some(CpuTopology::single_node(logical_processor_count()));
    }

    let node_count = highest_node.saturating_add(1) as usize;
    let mut numa_nodes = Vec::with_capacity(node_count);
    let mut processor_node_pairs = Vec::with_capacity(logical_processor_count());
    let mut logical_processors = 0usize;

    for raw_node in 0..=highest_node {
        if raw_node >= 1024 {
            continue;
        }
        // Total given the bound above; expressed as a conversion so no `as`
        // truncation is relied on.
        let Ok(node_index) = u16::try_from(raw_node) else {
            continue;
        };
        let mut affinity = GroupAffinity {
            mask: 0,
            group: 0,
            reserved: [0; 3],
        };
        // SAFETY: The API writes one GROUP_AFFINITY structure through a valid pointer.
        if unsafe { GetNumaNodeProcessorMaskEx(node_index, &mut affinity) } == 0
            || affinity.mask == 0
        {
            continue;
        }
        let node_id = NumaNodeId::new(raw_node);
        let mask = affinity.mask as u64;
        let mut processors = Vec::with_capacity(mask.count_ones() as usize);
        for bit in 0..64u32 {
            if (mask & (1u64 << bit)) != 0 {
                let system_processor = u32::from(affinity.group) * 64 + bit;
                if system_processor >= 32768 {
                    continue;
                }
                processors.push(system_processor);
                processor_node_pairs.push((system_processor, node_id));
                logical_processors = logical_processors.max(system_processor as usize + 1);
            }
        }
        numa_nodes.push(NumaNode {
            id: node_id,
            processors: processors.into_boxed_slice(),
            distances: build_default_distance_row(node_count, raw_node as usize),
            memory_tier: MemoryTier::Dram,
        });
    }

    if numa_nodes.is_empty() {
        return Some(CpuTopology::single_node(logical_processor_count()));
    }

    Some(CpuTopology {
        epoch: TopologyEpoch::INITIAL,
        node_to_index: build_node_to_index(&numa_nodes),
        adjacent_nodes: build_adjacent_nodes(&numa_nodes),
        numa_nodes: numa_nodes.into_boxed_slice(),
        processor_to_node: build_processor_to_node(
            logical_processors.max(1),
            &processor_node_pairs,
        ),
        logical_processors: logical_processors.max(1),
        cache_levels: detect_cache_levels(logical_processors.max(1)),
    })
}
