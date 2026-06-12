//! TPU topology query types.

use super::types::TpuDeviceProperties;
use crate::law::TopologyEpoch;

/// TPU device topology snapshot.
///
/// Provider-fed: themis stays stateless law, so there is no `detect()` here.
/// TPU backends construct this from PJRT or runtime-reported device
/// attributes via [`TpuTopology::from_provider`]. Unreported fields are zero,
/// never inferred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TpuTopology {
    epoch: TopologyEpoch,
    properties: TpuDeviceProperties,
}

impl TpuTopology {
    /// Construct a snapshot from provider-reported TPU properties.
    #[must_use]
    pub const fn from_provider(properties: TpuDeviceProperties) -> Self {
        Self {
            epoch: TopologyEpoch::INITIAL,
            properties,
        }
    }

    /// Snapshot epoch.
    #[must_use]
    #[inline]
    pub const fn epoch(&self) -> TopologyEpoch {
        self.epoch
    }

    /// TPU core count.
    #[must_use]
    #[inline]
    pub const fn core_count(&self) -> u32 {
        self.properties.core_count
    }

    /// HBM capacity per TPU core in bytes.
    #[must_use]
    #[inline]
    pub const fn hbm_bytes_per_core(&self) -> u64 {
        self.properties.hbm_bytes_per_core
    }

    /// Total provider-reported HBM capacity in bytes.
    ///
    /// Uses saturating arithmetic so malformed provider input cannot wrap the
    /// capacity value.
    #[must_use]
    #[inline]
    pub const fn total_hbm_bytes(&self) -> u64 {
        self.properties
            .hbm_bytes_per_core
            .saturating_mul(self.properties.core_count as u64)
    }
}
