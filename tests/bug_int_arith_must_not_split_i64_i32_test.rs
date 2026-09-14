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

//! FAILING REPRO — int `%` / compare / add must not split i64 vs i32.
//!
//! Product tip api-check (LedgerKit `domain/recurring.wj`) on tip HEAD after
//! int_inference churn: `year % 4`, `m > 12`, `remaining > 0` → E0277.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/int_arith_must_not_split_i64_i32.wj");

#[test]
fn int_arith_must_not_split_i64_i32() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let mixed = rs.contains(" as i32") && rs.contains(" as i64")
        || rs.contains("% 4_i32")
        || rs.contains("% 4i32")
        || (rs.contains("i64") && rs.contains("i32") && rs.contains('%'));
    if !ok || mixed {
        eprintln!("RED P3.267 int i64/i32 split:\nok={ok}\n{rs}");
    }
    assert!(
        ok,
        "RED P3.267: int arith must cargo-check without i64/i32 split. Generated:\n{rs}"
    );
}
