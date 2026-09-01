//! Dense ranking of raw platform capability values into efficiency classes.
//!
//! Every backend reduces its platform data to one raw capability value per
//! logical processor, higher meaning more performant, and hands the table here.
//! Windows reports a small `EfficiencyClass` byte, Linux ARM reports a
//! `cpu_capacity` in the hundreds or thousands, and Linux Intel reports set
//! membership. Dense ranking is what makes those a single comparable type.

use crate::topology::types::EfficiencyClass;

/// Upper bound on distinct classes, set by the `u8` rank of `EfficiencyClass`.
const MAX_CLASSES: usize = 256;

/// Dense-ranks one raw capability value per processor into classes.
///
/// `raw[i]` is the platform's capability value for processor `i`, higher
/// meaning more performant. The result preserves the ordering of `raw` while
/// compressing whatever values the platform used into the contiguous range
/// `0..distinct_count`.
///
/// Returns `None` — typed absence, never a fabricated split — when the table is
/// empty or reports more distinct values than a rank can hold. A table of one
/// distinct value is a homogeneous host and ranks every processor
/// [`EfficiencyClass::LOWEST`]; that is a reported result, distinct from `None`.
pub(super) fn dense_ranks(raw: &[u32]) -> Option<Box<[EfficiencyClass]>> {
    if raw.is_empty() {
        return None;
    }

    let mut distinct: Vec<u32> = raw.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() > MAX_CLASSES {
        return None;
    }

    let classes = raw
        .iter()
        .map(|value| {
            // `distinct` is the sorted deduplication of `raw`, so the search
            // always lands on the value; both arms carry the same index.
            let rank = match distinct.binary_search(value) {
                Ok(rank) | Err(rank) => rank,
            };
            u8::try_from(rank).ok().map(EfficiencyClass::new)
        })
        .collect::<Option<Vec<_>>>()?;

    Some(classes.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::dense_ranks;
    use crate::topology::types::EfficiencyClass;

    fn ranks(raw: &[u32]) -> Vec<u8> {
        dense_ranks(raw)
            .expect("fixture reports at least one processor")
            .iter()
            .map(|class| class.rank())
            .collect()
    }

    #[test]
    fn homogeneous_values_collapse_to_the_lowest_rank() {
        assert_eq!(ranks(&[1, 1, 1, 1]), vec![0, 0, 0, 0]);
        assert_eq!(
            dense_ranks(&[7; 4]).expect("fixture reports processors")[0],
            EfficiencyClass::LOWEST
        );
    }

    #[test]
    fn sparse_platform_values_compress_while_preserving_order() {
        // Linux ARM `cpu_capacity`: two little cores at 462, two big at 1024.
        assert_eq!(ranks(&[462, 462, 1024, 1024]), vec![0, 0, 1, 1]);
        // Three tiers with arbitrary spacing keep their relative order.
        assert_eq!(ranks(&[100, 9000, 250, 100]), vec![0, 2, 1, 0]);
    }

    #[test]
    fn higher_raw_value_always_ranks_higher() {
        let table = dense_ranks(&[0, 1, 2]).expect("fixture reports processors");
        assert!(table[0] < table[1]);
        assert!(table[1] < table[2]);
    }

    #[test]
    fn empty_and_oversized_tables_are_absent() {
        assert_eq!(dense_ranks(&[]), None);
        let too_many: Vec<u32> = (0..257).collect();
        assert_eq!(dense_ranks(&too_many), None);
        let at_limit: Vec<u32> = (0..256).collect();
        assert!(dense_ranks(&at_limit).is_some());
    }
}
