//! Real sysfs-shaped filesystem fixtures for the production cache reader.

use super::detect;
use crate::topology::types::CacheLevel;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct CacheFixture {
    root: PathBuf,
}

impl CacheFixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "themis-cache-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated cache fixture");
        Self { root }
    }

    fn cache(&self, cpu: u32, index: u32, sharing: &str) {
        let path = self.root.join(format!("cpu{cpu}/cache/index{index}"));
        fs::create_dir_all(&path).expect("create cache index");
        for (name, value) in [
            ("level", "3\n"),
            ("size", "8192K\n"),
            ("type", "Unified\n"),
            ("coherency_line_size", "64\n"),
            ("shared_cpu_list", sharing),
        ] {
            fs::write(path.join(name), value).expect("write cache attribute");
        }
    }
}

impl Drop for CacheFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).expect("remove owned cache fixture");
    }
}

fn expected_cache(processors: &[u32]) -> CacheLevel {
    CacheLevel {
        level: 3,
        size_bytes: 8192 * 1024,
        line_bytes: Some(64),
        shared_processors: processors.into(),
    }
}

#[test]
fn complete_sparse_sharing_report_preserves_values_and_deduplicates_records() {
    let fixture = CacheFixture::new();
    fixture.cache(0, 3, "0-1,4-5\n");
    fixture.cache(4, 3, "0-1,4-5\n");
    assert_eq!(
        detect(&fixture.root),
        Some(vec![expected_cache(&[0, 1, 4, 5])].into_boxed_slice())
    );
}

#[test]
fn malformed_sharing_report_does_not_become_a_partial_cache() {
    let fixture = CacheFixture::new();
    fixture.cache(0, 3, "0-1,broken\n");
    assert_eq!(detect(&fixture.root), None);
}

#[test]
fn each_malformed_sharing_report_is_absent_at_the_provider_boundary() {
    let fixture = CacheFixture::new();
    for sharing in [
        "",
        "0,",
        "0,,1",
        "broken,0",
        "0,2-1",
        "0,32768",
        "0,4294967296",
        "0,0",
        "1,0",
        "0-2,2-3",
        "0,+1",
        "0, 1",
        "0,１",
    ] {
        fixture.cache(0, 3, sharing);
        assert_eq!(detect(&fixture.root), None, "{sharing:?}");
    }
}

#[test]
fn a_rejected_record_does_not_erase_an_independent_valid_cache() {
    let fixture = CacheFixture::new();
    fixture.cache(0, 3, "0-1,broken\n");
    fixture.cache(4, 3, "4-5\n");
    assert_eq!(
        detect(&fixture.root),
        Some(vec![expected_cache(&[4, 5])].into_boxed_slice())
    );
}
