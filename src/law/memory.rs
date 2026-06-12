//! Memory locality tiers.

/// Memory tier classification.
///
/// Host-allocatable tiers (`Dram`, `Hbm`, `Gddr`, `HostPinned`, `Device`,
/// `Persistent`) are valid allocation targets for allocators such as
/// mnemosyne. The device-side tiers `Registers` and `SharedMem` are
/// **budgeted, non-host-allocatable** (atlas ADR 0002): GPU compilers assign
/// registers and kernels declare shared memory at launch, so these variants
/// exist purely as the typed vocabulary for capacity queries and kernel
/// resource budgets (occupancy planning) — never as allocation requests.
/// [`MemoryTier::is_host_allocatable`] encodes the distinction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// Standard host DRAM.
    #[default]
    Dram,
    /// High-bandwidth memory (host- or device-attached HBM stacks).
    Hbm,
    /// Device-attached GDDR memory (discrete-GPU global memory that is not
    /// HBM; distinct bandwidth/latency law from `Hbm`).
    Gddr,
    /// Page-locked (pinned) host memory for DMA staging transfers.
    HostPinned,
    /// Device-local memory of an unspecified technology.
    Device,
    /// Persistent memory.
    Persistent,
    /// GPU register file capacity (budgeted; compiler-assigned, never
    /// host-allocated).
    Registers,
    /// GPU shared/local memory per compute unit (budgeted; declared at
    /// kernel launch, never host-allocated).
    SharedMem,
}

impl MemoryTier {
    /// Returns true when the tier is a valid host-side allocation target.
    ///
    /// `Registers` and `SharedMem` return false: they are budget/capacity
    /// vocabulary for occupancy planning, not allocatable address space.
    #[must_use]
    #[inline]
    pub const fn is_host_allocatable(self) -> bool {
        !matches!(self, Self::Registers | Self::SharedMem)
    }
}
