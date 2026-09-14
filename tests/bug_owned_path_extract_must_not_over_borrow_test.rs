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

//! FAILING REPRO — owned path extract formal must not receive `&String` at call site.
//!
//! Product tip api-check (LedgerKit routes.wj):
//!   `extract_mcp_tool_name(&invoke_path)` → E0308 expected String, found &String.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/owned_path_extract_must_not_over_borrow.wj");

#[test]
fn owned_path_extract_must_not_over_borrow() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let bad = rs.contains("extract_tool_name(&");
    if bad {
        eprintln!("RED P3.266 owned path extract over-borrow:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.266: owned path extract must not receive &arg. Generated:\n{rs}"
    );
    assert!(
        ok,
        "RED P3.266: owned path extract must cargo-check. Generated:\n{rs}"
    );
}
