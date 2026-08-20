//! Construct a GPU topology snapshot and compute occupancy metrics.
//!
//! `GpuTopology` is *provider-fed*: hephaestus reads device attributes from
//! wgpu or CUDA and constructs [`GpuDeviceProperties`] directly.  This
//! example hard-codes representative A100-like properties so it runs without
//! a physical GPU.
//!
//! The three occupancy metrics — max resident warps, register budget, and
//! shared-memory budget — are computed directly from the snapshot and can be
//! used by moirai's launch shaper to set block dimensions at kernel dispatch
//! time.

#![expect(
    clippy::print_stdout,
    reason = "book example: stdout is the demonstrated output"
)]

extern crate themis;

use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use themis::{GpuDeviceProperties, GpuTopology, MemoryTier};

fn main() {
    // Representative NVIDIA A100-80GB properties.
    let a100_props = GpuDeviceProperties {
        compute_units: NonZeroU32::new(108),
        warp_width: NonZeroU32::new(32),
        max_threads_per_unit: NonZeroU32::new(2048),
        registers_per_unit: NonZeroU32::new(65536),
        shared_mem_per_unit_bytes: NonZeroUsize::new(167_936),
        l2_bytes: NonZeroUsize::new(41_943_040),
        memory_tier: MemoryTier::Hbm,
        memory_bytes: NonZeroU64::new(85_899_345_920),
    };

    let a100 = GpuTopology::from_provider(a100_props);
    println!("=== A100 SXM4-80GB (representative) ===");
    println!(
        "compute units       : {}",
        a100.compute_units().map_or(0, NonZeroU32::get)
    );
    println!(
        "warp width          : {}",
        a100.warp_width().map_or(0, NonZeroU32::get)
    );
    println!(
        "max threads/unit    : {}",
        a100.max_threads_per_unit().map_or(0, NonZeroU32::get)
    );
    println!(
        "registers/unit      : {}",
        a100.registers_per_unit().map_or(0, NonZeroU32::get)
    );
    println!(
        "shared mem/unit     : {} KiB",
        a100.shared_mem_per_unit_bytes()
            .map_or(0, NonZeroUsize::get)
            / 1024
    );
    println!(
        "L2 cache            : {} MiB",
        a100.l2_bytes().map_or(0, NonZeroUsize::get) / (1024 * 1024)
    );
    println!("memory tier         : {:?}", a100.memory_tier());
    println!(
        "memory              : {} GiB",
        a100.memory_bytes().map_or(0, NonZeroU64::get) / (1024 * 1024 * 1024)
    );

    let max_warps = a100
        .max_resident_warps()
        .expect("all three capacities reported");
    println!("max resident warps  : {max_warps}");
    assert_eq!(
        max_warps,
        108 * 2048 / 32,
        "max_resident_warps = compute_units × max_threads_per_unit / warp_width"
    );

    // Register budget for a kernel using 32 registers per thread. Counts stay
    // in `u32` so the occupancy ratio converts to `f64` without precision loss.
    let registers_per_thread = 32u32;
    let regs_per_unit = a100.registers_per_unit().expect("reported").get();
    let warps_per_unit_reg_limited = regs_per_unit / (registers_per_thread * 32);
    let warp_slots_per_unit = f64::from(2048u32 / 32);
    println!("\nregister-limited occupancy at {registers_per_thread} regs/thread:");
    println!("  warps/unit          : {warps_per_unit_reg_limited}");
    println!(
        "  occupancy           : {:.1} %",
        f64::from(warps_per_unit_reg_limited) / warp_slots_per_unit * 100.0
    );

    // Shared-memory budget for 16 KiB per thread block (warp = 1 block here).
    let smem_per_block_bytes: usize = 16 * 1024;
    let smem_per_unit = a100.shared_mem_per_unit_bytes().expect("reported").get();
    let blocks_per_unit_smem = smem_per_unit / smem_per_block_bytes;
    println!("\nshared-memory-limited occupancy at {smem_per_block_bytes} B/block:");
    println!("  blocks/unit         : {blocks_per_unit_smem}");

    // memory_tier is Hbm — confirms host-allocatability.
    assert!(a100.memory_tier().is_host_allocatable());

    // Demonstrate a device with unknown capacities (wgpu adapter).
    let wgpu_props = GpuDeviceProperties {
        compute_units: None, // wgpu does not expose SM count
        warp_width: NonZeroU32::new(32),
        max_threads_per_unit: None,
        registers_per_unit: None,
        shared_mem_per_unit_bytes: NonZeroUsize::new(32_768),
        l2_bytes: None,
        memory_tier: MemoryTier::Device,
        memory_bytes: None,
    };
    let wgpu_topo = GpuTopology::from_provider(wgpu_props);
    assert!(
        wgpu_topo.max_resident_warps().is_none(),
        "max_resident_warps is None when any required capacity is unknown"
    );
    println!(
        "\nwgpu device: max_resident_warps = {:?} (expected None)",
        wgpu_topo.max_resident_warps()
    );
}
