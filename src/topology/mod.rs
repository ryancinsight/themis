//! CPU, GPU, and TPU topology representations.

mod cpu;
mod gpu;
mod tpu;
mod types;

pub use cpu::{CpuEfficiencyView, CpuTopology};
#[cfg(windows)]
pub use cpu::{ProcessorAffinityGroups, ProcessorGroupAffinity};
pub use gpu::GpuTopology;
pub use tpu::TpuTopology;
pub use types::{CacheLevel, EfficiencyClass, GpuDeviceProperties, NumaNode, TpuDeviceProperties};

// Test-only re-export of CPU table builders so the crate root can surface them
// via `pub use topology::{build_*}` under the `testing` feature. The builders
// are `pub` at `cpu::tables::*` but `cpu` is a private module at `topology`.
#[cfg(any(test, feature = "testing"))]
pub use cpu::{
    build_adjacent_nodes, build_default_distance_row, build_node_to_index, build_processor_to_node,
};
