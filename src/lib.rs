//! Typed placement law for Atlas memory and runtime crates.
//!
//! Themis owns placement vocabulary, not allocation or execution. Mnemosyne
//! consumes these types to choose memory locality. Moirai consumes these types
//! to choose worker and task locality.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

#[cfg(feature = "std")]
extern crate std;

mod law;
mod query;
mod topology;

pub use law::{LocalityDomainId, MemoryTier, NumaNodeId, PlacementHint, TopologyEpoch, WorkerId};
pub use query::{current_numa_node, current_processor, refresh_current_numa_node};
pub use topology::{CacheLevel, CpuTopology, NumaNode};
