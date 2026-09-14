#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — owned Vec<Custom> into view helper must not receive `&Vec`.
//!
//! Product tip api-check (LedgerKit postgres_invoice_repository.wj):
//!   `invoice_view_from_row(header, &lines)` → E0308 expected Vec, found &Vec.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/vec_custom_view_helper_must_not_over_borrow.wj");

#[test]
fn vec_custom_view_helper_must_not_over_borrow() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let bad = rs.contains("view_from(&");
    if bad || !ok {
        eprintln!("RED P3.266 Vec<Custom> view helper over-borrow:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.266: owned Vec<Custom> helper must not receive &arg. Generated:\n{rs}"
    );
    assert!(
        ok,
        "RED P3.266: owned Vec<Custom> helper must cargo-check. Generated:\n{rs}"
    );
}
