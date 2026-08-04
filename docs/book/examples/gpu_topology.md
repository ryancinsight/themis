# Example: GPU Topology and Occupancy

**Crate**: `themis`
**Source**: `examples/book_gpu_topology.rs`

`GpuTopology` is constructed by the device backend (hephaestus) from driver-
reported attributes and handed to the occupancy planner (moirai).  Themis
stays stateless: it owns the types but never queries the driver itself.  This
example hard-codes A100-like properties so it runs without a physical GPU.

## Source

```rust
{{#include ../../../examples/book_gpu_topology.rs}}
```

## Output

```text
=== A100 SXM4-80GB (representative) ===
compute units       : 108
warp width          : 32
max threads/unit    : 2048
registers/unit      : 65536
shared mem/unit     : 164 KiB
L2 cache            : 40 MiB
memory tier         : Hbm
memory              : 80 GiB
max resident warps  : 6912

register-limited occupancy at 32 regs/thread:
  warps/unit          : 64
  occupancy           : 100.0 %

shared-memory-limited occupancy at 16384 B/block:
  blocks/unit         : 10

wgpu device: max_resident_warps = None (expected None)
```

## What to notice

- Every capacity field in `GpuDeviceProperties` is `Option<NonZero*>`.
  A field the driver did not report is `None`; there is no sentinel zero that
  could silently produce a divide-by-zero in the occupancy formula.

- `max_resident_warps()` = 108 × 2048 / 32 = 6912.  The formula is
  `compute_units × max_threads_per_unit / warp_width`; if any of the three
  inputs is `None`, the method returns `None` rather than a guess.

- Register-limited occupancy at 32 registers per thread:
  `65536 / (32 × 32) = 64 warps/unit`, which equals the theoretical maximum
  of `2048 / 32 = 64 warps/unit` — so this kernel is register-bound at 100 %
  theoretical occupancy.  A kernel using 64 registers per thread would drop to
  50 %.

- The wgpu device has `compute_units = None` (wgpu's abstract API does not
  expose SM count), so `max_resident_warps()` returns `None`.  Callers can
  fall back to a conservative block size rather than crashing.

- `memory_tier = Hbm` on the A100 confirms that `is_host_allocatable()` is
  `true` and mnemosyne can use device-memory pools from the host side.
