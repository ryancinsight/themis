# 2. Memory Tiers

`MemoryTier` classifies a region of memory by its physical technology and
access characteristics.  It lives in `src/law/memory.rs`:

```rust,ignore
pub enum MemoryTier {
    #[default] Dram,
    Hbm,
    Gddr,
    HostPinned,
    Device,
    Persistent,
    Registers,
    SharedMem,
}
impl MemoryTier {
    pub const fn is_host_allocatable(self) -> bool
}
```

## Variants

**`Dram`** (the default) is standard host DRAM attached to the memory
controller of a CPU socket.  It is the baseline tier that mnemosyne uses when
no other preference is specified.

**`Hbm`** is high-bandwidth memory stacked on or beside a processor die —
for example, HBM2e on an NVIDIA A100 or HBM2 integrated into an AMD EPYC
Genoa-X.  Bandwidth is substantially higher than DRAM; capacity is smaller.
On the GPU side, `Hbm` corresponds to the device-local pool.

**`Gddr`** is discrete GPU global memory using GDDR technology (e.g. GDDR6X
on an RTX 4090), as distinct from HBM.  Both `Hbm` and `Gddr` are
device-local memory, but the distinction matters for bandwidth-locality
modelling and for matching allocations to the correct device pool.

**`HostPinned`** is page-locked host memory.  The OS will not page it out,
which makes it suitable as a DMA staging buffer between the CPU and a device.
It is allocatable by the host, but incurs physical page pressure.

**`Device`** is device-local memory of unspecified technology.  Use this
tier when the device type or memory technology is not known at compile time.
Consumers that distinguish `Hbm` from `Gddr` should treat `Device` as a
lower-confidence match.

**`Persistent`** is byte-addressable non-volatile memory such as Intel Optane
DC.  Latency is higher than DRAM; capacity per DIMM slot is larger.  The OS
and firmware expose it as a NUMA node in DAX or fsdax mode.

**`Registers`** and **`SharedMem`** are GPU-internal resources managed
entirely by the compiler and runtime, not by the host allocator.

## `is_host_allocatable`

```rust,ignore
pub const fn is_host_allocatable(self) -> bool
```

Returns `false` for `Registers` and `SharedMem`; returns `true` for all
other variants.  The method is `const`, so it can be evaluated in static
assertions or `const` contexts.

A `PlacementHint::Tier(t)` hint with `!t.is_host_allocatable()` indicates a
programming error at the call site: registers and GPU shared memory are not
memory regions a host allocator can return pointers into.  The GPU compiler
assigns registers to variables; the kernel launch parameter declares the
shared memory size per thread block.  Themis exposes them as `MemoryTier`
variants so that mnemosyne can read them as occupancy budgets — not as
allocatable pools.

## Relationship to `PlacementHint`

`MemoryTier` appears inside `PlacementHint::Tier(MemoryTier)`.  A caller that
needs HBM without knowing the NUMA node writes
`PlacementHint::Tier(MemoryTier::Hbm)`; the allocator resolves the tier to a
concrete NUMA node or device pool using the topology snapshot.  See
[Placement Hints](placement_hints.md) for the full resolution flow.
