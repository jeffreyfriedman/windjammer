#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-235 multipass codegen: demoted `&mut DenseCsr` into owned `distances_to_map` must clone.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb235_mut_ref_csr_into_owned_must_clone.wj");

#[test]
fn wdb235_codegen_mut_ref_csr_into_owned_distances_to_map_must_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let callee_owned = rs.contains("fn distances_to_map(csr: DenseCsr")
        || rs.contains("distances_to_map(csr: DenseCsr");
    let caller_demoted = rs.contains("fn run_batch(csr: &mut DenseCsr")
        || rs.contains("run_batch(csr: &mut DenseCsr");
    let bare = rs.contains("distances_to_map(csr,") && !rs.contains("distances_to_map(csr.clone(),");
    let bad = callee_owned && caller_demoted && bare;
    eprintln!(
        "WDB-235 codegen owned={} demoted={} bare={} bad={}\n{rs}",
        callee_owned, caller_demoted, bare, bad
    );
    assert!(
        !bad,
        "WDB-235: demoted &mut csr into owned distances_to_map must clone. Generated:\n{rs}"
    );
    assert!(ok, "WDB-235 fixture must cargo-check. Generated:\n{rs}");
}
