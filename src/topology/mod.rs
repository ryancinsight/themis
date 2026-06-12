//! CPU and GPU topology representations.

mod cpu;
mod gpu;
mod types;

#[cfg(test)]
mod tests;

pub use cpu::CpuTopology;
pub use gpu::GpuTopology;
pub use types::{CacheLevel, GpuDeviceProperties, NumaNode};
