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
pub(in crate::topology) fn build_default_distance_row(
    node_count: usize,
    from_index: usize,
) -> Box<[u32]> {
    (0..node_count)
        .map(|to_index| default_distance(from_index, to_index))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

#[cfg(any(test, feature = "std"))]
pub(in crate::topology) fn build_processor_to_node(
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

pub(in crate::topology) fn build_node_to_index(nodes: &[NumaNode]) -> Box<[Option<usize>]> {
    let max_node = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let mut node_to_index = vec![None; max_node + 1];
    for (index, node) in nodes.iter().enumerate() {
        node_to_index[node.id.index()] = Some(index);
    }
    node_to_index.into_boxed_slice()
}

pub(in crate::topology) fn build_adjacent_nodes(nodes: &[NumaNode]) -> Box<[Box<[NumaNodeId]>]> {
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
                        .unwrap_or(default_distance(from_index, to_index));
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
