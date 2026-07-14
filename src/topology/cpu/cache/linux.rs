//! Linux sysfs cache-index discovery.

use crate::topology::cpu::parse_cpu_list;
use crate::topology::types::CacheLevel;
use std::fs;
use std::path::Path;

const CPU_ROOT: &str = "/sys/devices/system/cpu";
const MAX_CPU_ENTRIES: usize = 32_768;
const MAX_CACHE_INDICES: usize = 64;
const MAX_CACHE_LEVELS: usize = 512;

pub(super) fn detect() -> Option<Box<[CacheLevel]>> {
    let mut cpu_paths = fs::read_dir(CPU_ROOT)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let processor = name.strip_prefix("cpu")?.parse::<usize>().ok()?;
            (processor < MAX_CPU_ENTRIES).then_some((processor, entry.path()))
        })
        .collect::<Vec<_>>();
    cpu_paths.sort_unstable_by_key(|(processor, _)| *processor);

    let mut levels = Vec::new();
    for (_, cpu_path) in cpu_paths {
        let cache_path = cpu_path.join("cache");
        let Ok(indices) = fs::read_dir(cache_path) else {
            continue;
        };
        let mut index_paths = indices
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let index = name.strip_prefix("index")?.parse::<usize>().ok()?;
                (index < MAX_CACHE_INDICES).then_some((index, entry.path()))
            })
            .collect::<Vec<_>>();
        index_paths.sort_unstable_by_key(|(index, _)| *index);

        for (_, index_path) in index_paths {
            let Some(level) = read_level(&index_path) else {
                continue;
            };
            let Some(size_bytes) = read_cache_size(&index_path) else {
                continue;
            };
            let Some(shared_processors) = read_shared_processors(&index_path) else {
                continue;
            };
            let line_bytes = read_positive_usize(index_path.join("coherency_line_size"));
            let candidate = CacheLevel {
                level,
                size_bytes,
                line_bytes,
                shared_processors,
            };
            if !levels
                .iter()
                .any(|existing: &CacheLevel| existing == &candidate)
            {
                levels.push(candidate);
                if levels.len() >= MAX_CACHE_LEVELS {
                    return Some(levels.into_boxed_slice());
                }
            }
        }
    }

    (!levels.is_empty()).then(|| levels.into_boxed_slice())
}

fn read_level(index_path: &Path) -> Option<u32> {
    let level = fs::read_to_string(index_path.join("level"))
        .ok()?
        .trim()
        .parse::<u32>()
        .ok()?;
    (1..=3).contains(&level).then_some(level)
}

fn read_cache_size(index_path: &Path) -> Option<usize> {
    parse_cache_size(&fs::read_to_string(index_path.join("size")).ok()?)
}

fn read_shared_processors(index_path: &Path) -> Option<Box<[u32]>> {
    let processors = parse_cpu_list(&fs::read_to_string(index_path.join("shared_cpu_list")).ok()?);
    (!processors.is_empty()).then(|| processors.into_boxed_slice())
}

fn read_positive_usize(path: impl AsRef<Path>) -> Option<usize> {
    let value = fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()?;
    (value > 0).then_some(value)
}

fn parse_cache_size(value: &str) -> Option<usize> {
    let value = value.trim();
    let (digits, multiplier) = match value.as_bytes().last().copied() {
        Some(b'K' | b'k') => (&value[..value.len().saturating_sub(1)], 1024u64),
        Some(b'M' | b'm') => (&value[..value.len().saturating_sub(1)], 1024u64.pow(2)),
        Some(b'G' | b'g') => (&value[..value.len().saturating_sub(1)], 1024u64.pow(3)),
        _ => (value, 1),
    };
    let bytes = digits.trim().parse::<u64>().ok()?.checked_mul(multiplier)?;
    usize::try_from(bytes).ok().filter(|bytes| *bytes > 0)
}

#[cfg(test)]
mod tests {
    use super::parse_cache_size;

    #[test]
    fn parses_sysfs_cache_size_units() {
        assert_eq!(parse_cache_size("32K"), Some(32 * 1024));
        assert_eq!(parse_cache_size("1M"), Some(1024 * 1024));
        assert_eq!(parse_cache_size("2G"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_cache_size("32768"), Some(32_768));
    }

    #[test]
    fn rejects_unknown_or_zero_cache_sizes() {
        assert_eq!(parse_cache_size("0"), None);
        assert_eq!(parse_cache_size("32KB"), None);
        assert_eq!(parse_cache_size("not-a-size"), None);
    }
}
