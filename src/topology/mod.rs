//! CPU, GPU, and TPU topology representations.

mod cpu;
mod gpu;
mod tpu;
mod types;

#[cfg(test)]
mod tests;

pub use cpu::CpuTopology;
pub use gpu::GpuTopology;
pub use tpu::TpuTopology;
pub use types::{CacheLevel, GpuDeviceProperties, NumaNode, TpuDeviceProperties};

#[cfg(test)]
pub(crate) use cpu::{build_adjacent_nodes, build_node_to_index, build_processor_to_node};
