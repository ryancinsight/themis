//! Linux core efficiency-class discovery.
//!
//! Prefers the Intel hybrid CPU-type directories, which name their members
//! directly, and falls back to the ARM `cpu_capacity` scalar. Parsing either
//! surface is [`super::sysfs`]; this file is only the reads that produce it.

use super::sysfs::{capacities_from_cpu_capacity, capacities_from_cpu_types, CPU_TYPE_NAMES};
use super::{classes_from_capacities, MAX_PROCESSOR_ID};
use crate::topology::types::EfficiencyClass;
use std::fs;

const CPU_ROOT: &str = "/sys/devices/system/cpu";

pub(super) fn detect(logical_processors: usize) -> Option<Box<[EfficiencyClass]>> {
    if logical_processors == 0 || logical_processors > MAX_PROCESSOR_ID {
        return None;
    }

    let type_lists: Vec<Option<String>> = CPU_TYPE_NAMES
        .iter()
        .map(|name| fs::read_to_string(format!("{CPU_ROOT}/types/{name}/cpulist")).ok())
        .collect();
    let type_lists: Vec<Option<&str>> = type_lists
        .iter()
        .map(|list| list.as_deref())
        .collect::<Vec<_>>();
    if let Some(capacities) = capacities_from_cpu_types(&type_lists, logical_processors) {
        return classes_from_capacities(&capacities, logical_processors);
    }

    let capacity_files: Vec<Option<String>> = (0..logical_processors)
        .map(|processor| fs::read_to_string(format!("{CPU_ROOT}/cpu{processor}/cpu_capacity")).ok())
        .collect();
    let capacity_files: Vec<Option<&str>> = capacity_files
        .iter()
        .map(|contents| contents.as_deref())
        .collect::<Vec<_>>();
    let capacities = capacities_from_cpu_capacity(&capacity_files)?;
    classes_from_capacities(&capacities, logical_processors)
}
