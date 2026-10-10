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

//! Regression: module file owned csr clone into mut ref must reborrow.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str =
    include_str!("fixtures/library_multipass/wdb217_mut_ref_csr_reborrow.wj");

#[test]
fn codegen_mut_ref_formal_must_reborrow_not_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let bad = rs.contains("touch(csr.clone())") || rs.contains("touch(&csr)");
    assert!(
        !bad,
        "&mut formal into &mut callee must reborrow bare csr. Generated:\n{rs}"
    );
    assert!(
        rs.contains("touch(csr)"),
        "expected bare reborrow call. Generated:\n{rs}"
    );
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

