//! CPU, GPU, and TPU topology representations.

mod cpu;
mod gpu;
mod tpu;
mod types;

/// Exclusive upper bound on NUMA node ids across every backend.
///
/// Processor ids are `u32`, and a NUMA node id is bounded far below that: the
/// dense index tables, the adjacency tables, and the placement-region tag
/// bitmap all size themselves from this cap, so a malformed platform report can
/// never drive an allocation past it. Every consumer of the number — the table
/// builders, all three detection backends, and the locality probes — shares
/// this one value.
pub(crate) const MAX_NUMA_NODE_IDS: usize = 1024;

#[cfg(feature = "std")]
pub use cpu::{bind_current_thread, BindError};
pub use cpu::{CpuEfficiencyView, CpuSmtView, CpuTopology};
#[cfg(windows)]
pub use cpu::{ProcessorAffinityGroups, ProcessorGroupAffinity};
pub use gpu::GpuTopology;
pub use tpu::TpuTopology;
pub use types::{
    CacheLevel, CoreId, EfficiencyClass, GpuDeviceProperties, NumaNode, TpuDeviceProperties,
};

// Test-only re-export of CPU table builders so the crate root can surface them
// via `pub use topology::{build_*}` under the `testing` feature. The builders
// are `pub` at `cpu::tables::*` but `cpu` is a private module at `topology`.
#[cfg(any(test, feature = "testing"))]
pub use cpu::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
};
