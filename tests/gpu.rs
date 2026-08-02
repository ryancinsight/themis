//! GPU topology integration tests.

use core::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

use themis::{GpuDeviceProperties, GpuTopology, MemoryTier, TopologyEpoch};

fn sample_properties() -> GpuDeviceProperties {
    GpuDeviceProperties {
        compute_units: NonZeroU32::new(46),
        warp_width: NonZeroU32::new(32),
        max_threads_per_unit: NonZeroU32::new(1536),
        registers_per_unit: NonZeroU32::new(65536),
        shared_mem_per_unit_bytes: NonZeroUsize::new(102_400),
        l2_bytes: NonZeroUsize::new(4 * 1024 * 1024),
        memory_tier: MemoryTier::Gddr,
        memory_bytes: NonZeroU64::new(8 * 1024 * 1024 * 1024),
    }
}

#[test]
fn provider_snapshot_round_trips_every_field() {
    let topology = GpuTopology::from_provider(sample_properties());
    assert_eq!(topology.compute_units(), NonZeroU32::new(46));
    assert_eq!(topology.warp_width(), NonZeroU32::new(32));
    assert_eq!(topology.max_threads_per_unit(), NonZeroU32::new(1536));
    assert_eq!(topology.registers_per_unit(), NonZeroU32::new(65536));
    assert_eq!(
        topology.shared_mem_per_unit_bytes(),
        NonZeroUsize::new(102_400)
    );
    assert_eq!(topology.l2_bytes(), NonZeroUsize::new(4 * 1024 * 1024));
    assert_eq!(topology.memory_tier(), MemoryTier::Gddr);
    assert_eq!(
        topology.memory_bytes(),
        NonZeroU64::new(8 * 1024 * 1024 * 1024)
    );
    assert_eq!(topology.epoch(), TopologyEpoch::INITIAL);
}

#[test]
fn max_resident_warps_is_units_times_threads_over_width() {
    let topology = GpuTopology::from_provider(sample_properties());
    // 46 * 1536 / 32 = 2208
    assert_eq!(topology.max_resident_warps(), Some(2208));

    // An unreported capacity makes the occupancy product unknowable, not
    // zero: the type carries the absence.
    let mut unreported_width = sample_properties();
    unreported_width.warp_width = None;
    assert_eq!(
        GpuTopology::from_provider(unreported_width).max_resident_warps(),
        None
    );
}

#[test]
fn budgeted_tiers_are_not_host_allocatable() {
    let host_allocatable = [
        MemoryTier::Registers.is_host_allocatable(),
        MemoryTier::SharedMem.is_host_allocatable(),
        MemoryTier::Gddr.is_host_allocatable(),
        MemoryTier::HostPinned.is_host_allocatable(),
        MemoryTier::Hbm.is_host_allocatable(),
        MemoryTier::Dram.is_host_allocatable(),
    ];

    assert_eq!(host_allocatable, [false, false, true, true, true, true]);
}
