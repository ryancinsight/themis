//! Topology structural types.

mod cpu;
mod gpu;
mod tpu;

pub use cpu::{CacheLevel, NumaNode};
pub use gpu::GpuDeviceProperties;
pub use tpu::TpuDeviceProperties;
