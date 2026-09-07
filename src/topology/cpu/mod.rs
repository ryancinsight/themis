//! CPU topology snapshot and accessors.

#[cfg(windows)]
mod affinity;
mod cache;
// Shared by the Linux NUMA and cache backends and by the Intel hybrid CPU-type
// parser. The parser's fixtures also run in the Windows test build, which is
// why `test` widens the gate there but nowhere else.
#[cfg(all(feature = "std", any(target_os = "linux", all(test, windows))))]
mod cpulist;
mod detect;
#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
mod efficiency;
mod efficiency_view;
#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
mod smt;
mod smt_view;
mod tables;
mod topology;

// Both detection helpers exist only where a backend reads them; on a target
// with neither they would be dead code under the lint floor.
#[cfg(windows)]
pub use affinity::{ProcessorAffinityGroups, ProcessorGroupAffinity};
#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
pub(crate) use cache::detect_cache_levels;
#[cfg(all(feature = "std", any(target_os = "linux", all(test, windows))))]
pub(crate) use cpulist::parse_cpu_list;
#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
pub(crate) use efficiency::detect_efficiency_classes;
pub use efficiency_view::CpuEfficiencyView;
#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
pub(crate) use smt::detect_core_ids;
pub use smt_view::CpuSmtView;
pub use tables::{build_adjacent_nodes, build_node_to_index};
#[cfg(any(test, all(feature = "std", any(windows, target_os = "linux"))))]
pub use tables::{build_default_distance_row, build_processor_to_node};
pub use tables::{LOCAL_DISTANCE, REMOTE_DISTANCE};
pub(crate) use topology::logical_processor_count;
pub use topology::CpuTopology;

/// Exclusive upper bound on logical processor ids across every backend.
///
/// Processor ids are `u32` throughout the crate; this is the tighter bound the
/// detection paths enforce so a malformed platform mask cannot size an
/// allocation. Every consumer is a platform detection path.
#[cfg(all(feature = "std", any(windows, target_os = "linux")))]
pub(crate) const MAX_PROCESSOR_ID: usize = 32_768;

// The efficiency surface is std-only, as is `CpuTopology::detect`; these
// tests allocate, so they build alongside it rather than under bare
// `cfg(test)`.
#[cfg(all(test, feature = "std"))]
mod tests;
