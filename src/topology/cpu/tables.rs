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
