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
))]

//! Regression: Copy formals must stay owned (i64, @derive(Copy) structs).
//!
//! Delegating Copy params to `infer_parameter_ownership` must not default them to
//! Borrowed — that generates `&i64` / `&AppDeps` and breaks comparisons, unary ops,
//! and call sites that pass Copy values by value (financial-management-platform).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn copy_int_formal_used_in_comparisons_and_arithmetic_is_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "json_encode.wj",
        r#"
fn digit_char(d: int) -> string {
    if d == 0 { return "0" }
    if d == 1 { return "1" }
    return "9"
}

pub fn int_to_json(value: int) -> string {
    if value == 0 {
        return "0"
    }
    if value < 0 {
        return "-" + int_to_json(-value)
    }
    let mut n = value
    let mut digits = ""
    while n > 0 {
        let digit = n % 10
        digits = digit_char(digit) + digits
        n = n / 10
    }
    digits
}
"#,
    );
    test.add_file("mod.wj", "pub mod json_encode");

    let map = test.compile().expect("compile");
    let rs = map.get("json_encode.rs").expect("json_encode.rs");
    assert!(
        rs.contains("pub fn int_to_json(value: i64)") || rs.contains("pub fn int_to_json(value:i64)"),
        "int formal must be owned i64, not &i64. Got:\n{rs}"
    );
    assert!(
        !rs.contains("int_to_json(value: &i64)"),
        "must not borrow Copy int formal. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}

#[test]
fn derive_copy_struct_formal_passed_to_owned_callee_stays_owned() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "deps.wj",
        r#"
@derive(Copy)
pub struct AppDeps {
    pub tag: int,
}

pub fn default_deps() -> AppDeps {
    AppDeps { tag: 1 }
}
"#,
    );
    test.add_file(
        "handlers.wj",
        r#"
use crate::deps::AppDeps

pub fn fetch_accounts(deps: AppDeps, slug: string) -> int {
    let _ = deps.tag
    slug.len()
}
"#,
    );
    test.add_file(
        "routes.wj",
        r#"
use crate::deps::{AppDeps, default_deps}
use crate::handlers::fetch_accounts

pub fn handle() -> int {
    let deps = default_deps()
    fetch_accounts(deps, "demo")
}
"#,
    );
    test.add_file(
        "mod.wj",
        r#"
pub mod deps
pub mod handlers
pub mod routes
"#,
    );

    test.assert_compiles_without_error();
}
