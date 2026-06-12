//! TPU topology structural types.

/// Provider-supplied TPU device properties for [`crate::TpuTopology::from_provider`].
///
/// A plain field struct (not a builder): every field is required, and the
/// provider reads them directly from PJRT or the active TPU runtime. Fields
/// the provider does not report are zero, never fabricated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TpuDeviceProperties {
    /// TPU core count (0 when unreported).
    pub core_count: u32,
    /// HBM capacity per TPU core in bytes (0 when unreported).
    pub hbm_bytes_per_core: u64,
}
