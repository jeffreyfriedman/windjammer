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

//! FAILING REPRO — owned Vec<string> helper must not receive `&Vec` at call site.
//!
//! Product tip api-check (LedgerKit http_json.wj):
//!   `string_list_json(&controls)` → E0308 expected Vec, found &Vec.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/vec_string_helper_must_not_over_borrow.wj");

#[test]
fn vec_string_helper_must_not_over_borrow() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let bad = rs.contains("list_json(&");
    if bad {
        eprintln!("RED P3.266 Vec<string> helper over-borrow:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.266: owned Vec helper must not receive &arg. Generated:\n{rs}"
    );
    assert!(
        ok,
        "RED P3.266: owned Vec helper must cargo-check. Generated:\n{rs}"
    );
}
