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
//! Product tip api-check (LedgerKit `domain/recurring.wj`) on tip after
//! int_inference churn: `year % 4`, `m > 12`, `remaining > 0` → E0277.
//! Isolate may cargo-check while still emitting `year as i64 % 400_i32` — that
//! is still RED (width split); assert the emit shape, not only rustc ok.
//!
//! Tip transpile verified RED (2026-09-14): `year as i64 % 400_i32`.

#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/int_arith_must_not_split_i64_i32.wj");

fn int_width_split(rs: &str) -> bool {
    rs.contains("% 400_i32")
        || rs.contains("% 100_i32")
        || rs.contains("% 4_i32")
        || rs.contains("% 400i32")
        || rs.contains("% 4i32")
        || (rs.contains(" as i64 %") && rs.contains("_i32"))
}

#[test]
fn int_arith_must_not_split_i64_i32() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let split = int_width_split(&rs);
    if !ok || split {
        eprintln!("RED P3.267 int i64/i32 split:\nok={ok} split={split}\n{rs}");
    }
    assert!(
        !split,
        "RED P3.267: int `%` must not mix i64 lhs with i32 literal. Generated:\n{rs}"
    );
    assert!(
        ok,
        "RED P3.267: int arith must cargo-check without i64/i32 split. Generated:\n{rs}"
    );
}
