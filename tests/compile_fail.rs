#[test]
fn placement_proofs_reject_invalid_cell_construction() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
