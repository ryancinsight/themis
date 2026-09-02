//! SMT and efficiency views answered from reported tables.

use themis::{CoreId, CpuTopology};

#[test]
fn smt_view_answers_sibling_questions_from_a_reported_table() {
    let cores = [0, 0, 1, 1, 2].map(CoreId::new);
    let topology = CpuTopology::single_node(5).with_core_ids_for_test(Some(Box::new(cores)));
    let smt = topology.smt().expect("the fixture reports a core table");

    assert_eq!(smt.physical_core_count(), 3);
    assert!(smt.has_smt());
    assert_eq!(smt.core_of(3), Some(CoreId::new(1)));
    assert_eq!(smt.core_of(9), None);
    assert_eq!(smt.are_siblings(0, 1), Some(true));
    assert_eq!(smt.are_siblings(1, 2), Some(false));
    assert_eq!(
        smt.siblings_of(2).map(Iterator::collect::<Vec<u32>>),
        Some(vec![3])
    );
    assert_eq!(
        smt.siblings_of(4).map(Iterator::collect::<Vec<u32>>),
        Some(vec![])
    );
    assert_eq!(
        smt.processors_in_core(CoreId::new(0)).collect::<Vec<u32>>(),
        vec![0, 1]
    );
    assert_eq!(
        smt.one_processor_per_core().collect::<Vec<u32>>(),
        vec![0, 2, 4]
    );
}

#[test]
fn a_host_without_smt_reports_a_present_table_not_absence() {
    let cores = [0, 1, 2].map(CoreId::new);
    let topology = CpuTopology::single_node(3).with_core_ids_for_test(Some(Box::new(cores)));
    let smt = topology
        .smt()
        .expect("one processor per core is still a report");
    assert!(!smt.has_smt());
    assert_eq!(smt.physical_core_count(), 3);
    assert_eq!(
        smt.one_processor_per_core().collect::<Vec<u32>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        CpuTopology::single_node(3)
            .smt()
            .map(themis::CpuSmtView::physical_core_count),
        None
    );
}
