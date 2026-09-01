//! Linux sysfs parsing for per-processor capability values.
//!
//! Two independent sysfs surfaces report the same underlying fact, and neither
//! is present on a host that has nothing to report:
//!
//! - **Intel hybrid** publishes set membership under
//!   `/sys/devices/system/cpu/types/`, one directory per CPU type with a
//!   `cpulist` naming its members. The directory names are the kernel's
//!   `intel_core` (performance) and `intel_atom` (efficient).
//! - **ARM big.LITTLE** publishes a scalar per processor at
//!   `/sys/devices/system/cpu/cpuN/cpu_capacity`, higher meaning more capable.
//!
//! Both parse to the same shape — one raw capability value per processor,
//! higher meaning more performant — which [`super::rank::dense_ranks`] then
//! compresses. The parsers are pure functions over the file contents, compiled
//! on every target so their fixtures run in every CI leg; only the reads that
//! produce those contents are Linux-gated.

use crate::topology::cpu::parse_cpu_list;

/// Kernel CPU-type directory names, ordered from least to most performant.
///
/// The order is the ranking: `intel_atom` are the efficient cores and
/// `intel_core` the performance cores. Adding a tier means inserting it at its
/// place in this list, not adding a second boolean.
pub(super) const CPU_TYPE_NAMES: [&str; 2] = ["intel_atom", "intel_core"];

/// Builds raw capability values from CPU-type `cpulist` contents.
///
/// `type_lists[i]` is the `cpulist` of `CPU_TYPE_NAMES[i]`, or `None` when that
/// type directory is absent. The raw value of a processor is the index of the
/// type that claims it, so a host publishing only one type is homogeneous
/// rather than absent.
///
/// Returns `None` when no type claims any processor, when a processor is
/// claimed by two types, or when the claimed processors do not exactly cover
/// `0..logical_processors`. Partial coverage is absence: a class table with
/// holes would be a fabricated split over the processors it did cover.
pub(super) fn capacities_from_cpu_types(
    type_lists: &[Option<&str>],
    logical_processors: usize,
) -> Option<Vec<u32>> {
    let mut capacities = vec![None; logical_processors];
    for (type_index, list) in type_lists.iter().enumerate() {
        let Some(list) = list else { continue };
        let raw = u32::try_from(type_index).ok()?;
        for processor in parse_cpu_list(list) {
            let slot = capacities.get_mut(usize::try_from(processor).ok()?)?;
            if slot.is_some() {
                return None;
            }
            *slot = Some(raw);
        }
    }
    capacities.into_iter().collect()
}

/// Builds raw capability values from `cpu_capacity` file contents.
///
/// `capacity_files[i]` is the `cpuN/cpu_capacity` content for processor `i`, or
/// `None` when that file is absent. Returns `None` unless every processor
/// reports a positive capacity — the same total-coverage rule as above.
pub(super) fn capacities_from_cpu_capacity(capacity_files: &[Option<&str>]) -> Option<Vec<u32>> {
    if capacity_files.is_empty() {
        return None;
    }
    capacity_files
        .iter()
        .map(|contents| {
            let value = (*contents)?.trim().parse::<u32>().ok()?;
            (value > 0).then_some(value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{capacities_from_cpu_capacity, capacities_from_cpu_types, CPU_TYPE_NAMES};

    #[test]
    fn cpu_type_order_is_the_ranking_least_performant_first() {
        // The position of a name in this list is its raw capability value, so
        // reordering the list silently inverts every Linux Intel hybrid result.
        assert_eq!(CPU_TYPE_NAMES, ["intel_atom", "intel_core"]);
        let capacities = capacities_from_cpu_types(&[Some("0"), Some("1")], 2)
            .expect("both type lists cover every processor");
        let atom_index = 0;
        let core_index = 1;
        assert!(
            capacities[atom_index] < capacities[core_index],
            "{} must rank below {}",
            CPU_TYPE_NAMES[atom_index],
            CPU_TYPE_NAMES[core_index]
        );
    }

    #[test]
    fn intel_type_lists_rank_atom_below_core() {
        // 4 performance threads and 4 efficient threads, interleaved ids.
        let capacities = capacities_from_cpu_types(&[Some("4-7"), Some("0-3")], 8)
            .expect("both type lists cover every processor");
        assert_eq!(capacities, vec![1, 1, 1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn a_single_cpu_type_covering_every_processor_is_homogeneous() {
        let capacities = capacities_from_cpu_types(&[None, Some("0-3")], 4)
            .expect("one type list covers every processor");
        assert_eq!(capacities, vec![1, 1, 1, 1]);
    }

    #[test]
    fn partial_overlapping_and_out_of_range_type_lists_are_absent() {
        // Processor 3 is claimed by no type.
        assert_eq!(
            capacities_from_cpu_types(&[Some("0,1"), Some("2")], 4),
            None
        );
        // Processor 1 is claimed by both types.
        assert_eq!(
            capacities_from_cpu_types(&[Some("0,1"), Some("1,2,3")], 4),
            None
        );
        // A type names a processor the snapshot does not have.
        assert_eq!(capacities_from_cpu_types(&[None, Some("0-7")], 4), None);
        // Nothing reported at all.
        assert_eq!(capacities_from_cpu_types(&[None, None], 4), None);
        assert_eq!(capacities_from_cpu_types(&[], 4), None);
    }

    #[test]
    fn cpu_capacity_values_are_read_verbatim_for_ranking() {
        // A Cortex-A55/A78 pair: four little at 462, four big at 1024.
        let capacities = capacities_from_cpu_capacity(&[
            Some("462\n"),
            Some("462\n"),
            Some("462\n"),
            Some("462\n"),
            Some("1024\n"),
            Some("1024\n"),
            Some("1024\n"),
            Some("1024\n"),
        ])
        .expect("every processor reports a capacity");
        assert_eq!(capacities, vec![462, 462, 462, 462, 1024, 1024, 1024, 1024]);
    }

    #[test]
    fn a_uniform_cpu_capacity_is_homogeneous_not_absent() {
        assert_eq!(
            capacities_from_cpu_capacity(&[Some("1024"), Some("1024")]),
            Some(vec![1024, 1024])
        );
    }

    #[test]
    fn missing_malformed_and_zero_capacities_are_absent() {
        assert_eq!(capacities_from_cpu_capacity(&[Some("1024"), None]), None);
        assert_eq!(
            capacities_from_cpu_capacity(&[Some("1024"), Some("not-a-number")]),
            None
        );
        assert_eq!(
            capacities_from_cpu_capacity(&[Some("1024"), Some("0")]),
            None
        );
        assert_eq!(
            capacities_from_cpu_capacity(&[Some("1024"), Some("")]),
            None
        );
        assert_eq!(capacities_from_cpu_capacity(&[]), None);
    }
}
