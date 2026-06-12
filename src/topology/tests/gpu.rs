//! GPU topology unit tests.

use super::super::gpu::GpuTopology;
use super::super::types::GpuDeviceProperties;
use crate::law::{MemoryTier, TopologyEpoch};

fn sample_properties() -> GpuDeviceProperties {
    GpuDeviceProperties {
        compute_units: 46,
        warp_width: 32,
        max_threads_per_unit: 1536,
        registers_per_unit: 65536,
        shared_mem_per_unit_bytes: 102_400,
        l2_bytes: 4 * 1024 * 1024,
        memory_tier: MemoryTier::Gddr,
        memory_bytes: 8 * 1024 * 1024 * 1024,
    }
}

#[test]
fn provider_snapshot_round_trips_every_field() {
    let topology = GpuTopology::from_provider(sample_properties());
    assert_eq!(topology.compute_units(), 46);
    assert_eq!(topology.warp_width(), 32);
    assert_eq!(topology.max_threads_per_unit(), 1536);
    assert_eq!(topology.registers_per_unit(), 65536);
    assert_eq!(topology.shared_mem_per_unit_bytes(), 102_400);
    assert_eq!(topology.l2_bytes(), 4 * 1024 * 1024);
    assert_eq!(topology.memory_tier(), MemoryTier::Gddr);
    assert_eq!(topology.memory_bytes(), 8 * 1024 * 1024 * 1024);
    assert_eq!(topology.epoch(), TopologyEpoch::INITIAL);
}

#[test]
fn max_resident_warps_is_units_times_threads_over_width() {
    let topology = GpuTopology::from_provider(sample_properties());
    // 46 * 1536 / 32 = 2208
    assert_eq!(topology.max_resident_warps(), 2208);

    let mut zero_width = sample_properties();
    zero_width.warp_width = 0;
    assert_eq!(
        GpuTopology::from_provider(zero_width).max_resident_warps(),
        0
    );
}

#[test]
fn budgeted_tiers_are_not_host_allocatable() {
    assert!(!MemoryTier::Registers.is_host_allocatable());
    assert!(!MemoryTier::SharedMem.is_host_allocatable());
    assert!(MemoryTier::Gddr.is_host_allocatable());
    assert!(MemoryTier::HostPinned.is_host_allocatable());
    assert!(MemoryTier::Hbm.is_host_allocatable());
    assert!(MemoryTier::Dram.is_host_allocatable());
}
