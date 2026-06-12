//! Current processor and NUMA-node query functions.

mod cache;
mod platform;

#[cfg(test)]
mod tests;

pub use cache::{current_numa_node, refresh_current_numa_node};
pub use platform::{current_processor, try_current_numa_node};
