//! TPU topology integration tests.

use themis::{TopologyEpoch, TpuDeviceProperties, TpuTopology};

fn sample_properties() -> TpuDeviceProperties {
    TpuDeviceProperties {
        core_count: 8,
        hbm_bytes_per_core: 16 * 1024 * 1024 * 1024,
    }
}

#[test]
fn provider_snapshot_round_trips_every_field() {
    let topology = TpuTopology::from_provider(sample_properties());
    assert_eq!(topology.core_count(), 8);
    assert_eq!(topology.hbm_bytes_per_core(), 16 * 1024 * 1024 * 1024);
    assert_eq!(topology.total_hbm_bytes(), 128 * 1024 * 1024 * 1024);
    assert_eq!(topology.epoch(), TopologyEpoch::INITIAL);
}

#[test]
fn total_hbm_bytes_saturates_malformed_provider_capacity() {
    let topology = TpuTopology::from_provider(TpuDeviceProperties {
        core_count: 2,
        hbm_bytes_per_core: u64::MAX,
    });
    assert_eq!(topology.total_hbm_bytes(), u64::MAX);
}
