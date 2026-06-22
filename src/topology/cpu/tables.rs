//! Dense CPU topology lookup-table builders.

use super::NumaNode;
use crate::law::NumaNodeId;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub(in crate::topology) const LOCAL_DISTANCE: u32 = 10;
pub(in crate::topology) const REMOTE_DISTANCE: u32 = 20;

pub(in crate::topology) const fn default_distance(from_index: usize, to_index: usize) -> u32 {
    if from_index == to_index {
        LOCAL_DISTANCE
    } else {
        REMOTE_DISTANCE
    }
}

#[cfg(any(test, feature = "std"))]
pub(crate) fn build_default_distance_row(
    node_count: usize,
    from_index: usize,
) -> Box<[u32]> {
    (0..node_count)
        .map(|to_index| default_distance(from_index, to_index))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[cfg(any(test, feature = "std"))]
pub(crate) fn build_processor_to_node(
    logical_processors: usize,
    mappings: &[(u32, NumaNodeId)],
) -> Box<[NumaNodeId]> {
    let max_processor = mappings
        .iter()
        .map(|(processor, _)| *processor as usize)
        .max()
        .unwrap_or(0);
    let mut processor_to_node = vec![NumaNodeId::INVALID; logical_processors.max(max_processor + 1).max(1)];
    for (processor, node) in mappings {
        processor_to_node[*processor as usize] = *node;
    }
    processor_to_node.into_boxed_slice()
}

pub(crate) fn build_node_to_index(nodes: &[NumaNode]) -> Box<[usize]> {
    let max_node = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let mut node_to_index = vec![usize::MAX; max_node + 1];
    for (index, node) in nodes.iter().enumerate() {
        let node_idx = node.id.index();
        assert!(
            node_to_index[node_idx] == usize::MAX,
            "invariant check failed: duplicate NUMA node ID {} found in topology",
            node.id.get()
        );
        node_to_index[node_idx] = index;
    }
    node_to_index.into_boxed_slice()
}

pub(crate) fn build_adjacent_nodes(nodes: &[NumaNode]) -> Box<[NumaNodeId]> {
    let node_count = nodes.len();
    if node_count <= 1 {
        return Box::default();
    }
    let max_node_id = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let stride = node_count - 1;
    let mut flat = Vec::with_capacity(node_count * stride);
    const STACK_LIMIT: usize = 128;
    if node_count <= STACK_LIMIT {
        let mut adjacent = [(NumaNodeId::ZERO, 0u32); STACK_LIMIT];
        for (from_index, from_node) in nodes.iter().enumerate() {
            let mut count = 0;
            for (to_index, to_node) in nodes.iter().enumerate() {
                if to_index != from_index {
                    let idx = if from_node.distances.len() > max_node_id {
                        to_node.id.index()
                    } else {
                        to_index
                    };
                    let distance = from_node
                        .distances
                        .get(idx)
                        .copied()
                        .unwrap_or(default_distance(from_index, to_index));
                    adjacent[count] = (to_node.id, distance);
                    count += 1;
                }
            }
            adjacent[..count].sort_by_key(|(_, distance)| *distance);
            for &(node_id, _) in adjacent.iter().take(count) {
                flat.push(node_id);
            }
        }
    } else {
        for (from_index, from_node) in nodes.iter().enumerate() {
            let mut adjacent: Vec<(NumaNodeId, u32)> = nodes
                .iter()
                .enumerate()
                .filter(|(to_index, _)| *to_index != from_index)
                .map(|(to_index, to_node)| {
                    let idx = if from_node.distances.len() > max_node_id {
                        to_node.id.index()
                    } else {
                        to_index
                    };
                    let distance = from_node
                        .distances
                        .get(idx)
                        .copied()
                        .unwrap_or(default_distance(from_index, to_index));
                    (to_node.id, distance)
                })
                .collect();
            adjacent.sort_by_key(|(_, distance)| *distance);
            for (node_id, _) in adjacent {
                flat.push(node_id);
            }
        }
    }
    flat.into_boxed_slice()
}
