//! Dense CPU topology lookup-table builders.

use super::super::types::NumaNode;
use crate::law::NumaNodeId;
use crate::topology::MAX_NUMA_NODE_IDS;

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

#[cfg(not(feature = "std"))]
use alloc::vec;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

pub const LOCAL_DISTANCE: u32 = 10;
pub const REMOTE_DISTANCE: u32 = 20;

pub(in crate::topology) const fn default_distance(from_index: usize, to_index: usize) -> u32 {
    if from_index == to_index {
        LOCAL_DISTANCE
    } else {
        REMOTE_DISTANCE
    }
}

/// Builds the default distance row for `from_index`: `LOCAL_DISTANCE` to self,
/// `REMOTE_DISTANCE` to every other node. Used when a NUMA node lacks an
/// explicit distances vector.
#[cfg(any(test, all(feature = "std", any(windows, target_os = "linux"))))]
#[must_use]
pub fn build_default_distance_row(node_count: usize, from_index: usize) -> Box<[u32]> {
    (0..node_count)
        .map(|to_index| default_distance(from_index, to_index))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

/// Builds the processor-to-NUMA-node lookup table from `mappings`.
///
/// Returns a `Box<[NumaNodeId]>` of length `max(logical_processors, max_processor + 1, 1)`
/// with `NumaNodeId::INVALID` for any processor not in `mappings`. Asserts the
/// resolved length stays within the 32768-processor hard cap to bound
/// allocation under adversarial input.
///
/// # Panics
///
/// Panics if the resolved table length exceeds the 32768-processor hard cap.
#[cfg(any(test, all(feature = "std", any(windows, target_os = "linux"))))]
#[must_use]
pub fn build_processor_to_node(
    logical_processors: usize,
    mappings: &[(u32, NumaNodeId)],
) -> Box<[NumaNodeId]> {
    let max_processor = mappings
        .iter()
        .map(|(processor, _)| *processor as usize)
        .max()
        .unwrap_or(0);
    let len = logical_processors.max(max_processor + 1).max(1);
    assert!(
        len <= 32768,
        "invariant check failed: processor count {len} exceeds maximum limit of 32768"
    );
    let mut processor_to_node = vec![NumaNodeId::INVALID; len];
    for (processor, node) in mappings {
        processor_to_node[*processor as usize] = *node;
    }
    processor_to_node.into_boxed_slice()
}

/// Builds the NUMA-node-id to dense-index lookup (length `max_node_id + 1`,
/// `usize::MAX` sentinel for absent IDs). Asserts the max node ID is below the
/// 1024-node hard cap and rejects duplicate node IDs to bound allocation and
/// preserve the index-uniqueness invariant.
///
/// # Panics
///
/// Panics if the maximum node id reaches the 1024-node hard cap, or if two
/// nodes share an id.
#[must_use]
pub fn build_node_to_index(nodes: &[NumaNode]) -> Box<[usize]> {
    let max_node = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    assert!(
        max_node < MAX_NUMA_NODE_IDS,
        "invariant check failed: NUMA node ID {max_node} exceeds maximum limit of {MAX_NUMA_NODE_IDS}"
    );
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

/// Resolves the distance from `from_index` to `to` under the shared NUMA
/// distance-row lookup rule.
///
/// A distance row is indexed **dense-by-node-id** when it is long enough to
/// cover the largest node id in the topology, and **compact-by-position**
/// otherwise; a missing entry falls back to [`default_distance`]. This is the
/// single rule used by [`build_adjacent_nodes`] and by
/// [`CpuTopology::distance`](crate::CpuTopology::distance).
#[inline]
pub(crate) fn distance_from_row(
    distances: &[u32],
    max_node_id: usize,
    to: NumaNodeId,
    to_index: usize,
    from_index: usize,
) -> u32 {
    let idx = if distances.len() > max_node_id {
        to.index()
    } else {
        to_index
    };
    distances
        .get(idx)
        .copied()
        .unwrap_or(default_distance(from_index, to_index))
}

/// Fills one source node's adjacency row into `out`, nearest first, returning
/// how many entries were written.
///
/// `out` must have room for every node other than `from_index` — that is, at
/// least `nodes.len() - 1` entries. The caller picks the scratch storage (a
/// fixed stack array for small topologies, one reused heap `Vec` otherwise), so
/// both arms of [`build_adjacent_nodes`] share this single implementation of
/// the row semantics.
fn fill_adjacency_row(
    from_index: usize,
    from_node: &NumaNode,
    nodes: &[NumaNode],
    max_node_id: usize,
    out: &mut [(NumaNodeId, u32)],
) -> usize {
    let mut count = 0;
    for (to_index, to_node) in nodes.iter().enumerate() {
        if to_index != from_index {
            let distance = distance_from_row(
                &from_node.distances,
                max_node_id,
                to_node.id,
                to_index,
                from_index,
            );
            out[count] = (to_node.id, distance);
            count += 1;
        }
    }
    out[..count].sort_by_key(|(_, distance)| *distance);
    count
}

/// Builds the per-node adjacency list as a flat `Box<[NumaNodeId]>` of length
/// `node_count * (node_count - 1)`: for each source node, the other nodes
/// sorted by distance (closest first). Uses the explicit `distances` vector
/// when present, falling back to `default_distance` otherwise.
#[must_use]
pub fn build_adjacent_nodes(nodes: &[NumaNode]) -> Box<[NumaNodeId]> {
    const STACK_LIMIT: usize = 128;

    let node_count = nodes.len();
    if node_count <= 1 {
        return Box::default();
    }
    let max_node_id = nodes.iter().map(|node| node.id.index()).max().unwrap_or(0);
    let stride = node_count - 1;
    let mut flat = Vec::with_capacity(node_count * stride);
    if node_count <= STACK_LIMIT {
        let mut adjacent = [(NumaNodeId::ZERO, 0u32); STACK_LIMIT];
        for (from_index, from_node) in nodes.iter().enumerate() {
            let count =
                fill_adjacency_row(from_index, from_node, nodes, max_node_id, &mut adjacent);
            for &(node_id, _) in adjacent.iter().take(count) {
                flat.push(node_id);
            }
        }
    } else {
        let mut adjacent = vec![(NumaNodeId::ZERO, 0u32); stride];
        for (from_index, from_node) in nodes.iter().enumerate() {
            let count =
                fill_adjacency_row(from_index, from_node, nodes, max_node_id, &mut adjacent);
            for &(node_id, _) in adjacent.iter().take(count) {
                flat.push(node_id);
            }
        }
    }
    flat.into_boxed_slice()
}
