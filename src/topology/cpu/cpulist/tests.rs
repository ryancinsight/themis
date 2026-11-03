use super::{parse_cpu_list, parse_cpu_set, MAX_PROCESSOR_ID};

#[test]
fn every_small_bitmap_round_trips_through_decimal_ranges() {
    // All nonempty subsets of eight processor ids, including adjacent and
    // separated runs. Membership comes directly from the bitmap oracle.
    for bitmap in 1..=255u32 {
        let expected: Vec<u32> = (0..8).filter(|id| bitmap & (1 << id) != 0).collect();
        let mut ranges = Vec::new();
        let mut index = 0;
        while index < expected.len() {
            let start = expected[index];
            let mut end = start;
            index += 1;
            while index < expected.len() && expected[index] == end + 1 {
                end = expected[index];
                index += 1;
            }
            ranges.push(if start == end {
                start.to_string()
            } else {
                format!("{start}-{end}")
            });
        }
        let report = format!("{}\n", ranges.join(","));
        assert_eq!(parse_cpu_set(&report), Some(expected), "{report:?}");
    }
}

#[test]
fn malformed_sets_are_rejected_without_retaining_valid_tokens() {
    for report in [
        "",
        "\n",
        "0,",
        ",0",
        "0,,1",
        "0,broken",
        "broken,0",
        "0,2-1",
        "0,-1",
        "0,1-",
        "0,1-2-3",
        "0,+1",
        "0, 1",
        "0,１",
        "0,4294967296",
        "0,32768",
        "0,32767-32768",
        "1,0",
        "0,0",
        "0-2,2-3",
        "0-3,1-2",
        "0\n1",
    ] {
        assert_eq!(parse_cpu_set(report), None, "{report:?}");
    }
}

#[test]
fn allocation_boundary_is_complete_and_does_not_hide_a_bad_suffix() {
    let highest = u32::try_from(MAX_PROCESSOR_ID - 1).expect("CPU bound fits u32");
    assert_eq!(parse_cpu_set(&highest.to_string()), Some(vec![highest]));
    let report = format!("0-{highest}");
    let expected: Vec<u32> = (0..=highest).collect();
    assert_eq!(parse_cpu_set(&report), Some(expected));
    for suffix in ["broken", "32768", "0"] {
        assert_eq!(parse_cpu_set(&format!("{report},{suffix}")), None);
    }
}

#[test]
fn tolerant_consumers_keep_their_existing_token_and_capacity_policy() {
    assert_eq!(
        parse_cpu_list("0-1,broken,4,+5,7-6,32768"),
        vec![0, 1, 4, 5]
    );
    assert_eq!(
        parse_cpu_list("3,1,1,0-2,,+4-+5"),
        vec![3, 1, 1, 0, 1, 2, 4, 5]
    );
    assert_eq!(parse_cpu_list(""), Vec::<u32>::new());
    let highest = u32::try_from(MAX_PROCESSOR_ID - 1).expect("CPU bound fits u32");
    let mut expected = vec![highest];
    expected.extend(0..highest);
    assert_eq!(
        parse_cpu_list(&format!("{highest},0-{highest},2")),
        expected
    );
}
