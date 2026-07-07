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

//! Runtime std modules (`strings`, `db`, `jwt`, …) take `AsRef<str>` / `&str` in Rust.
//! String literals at those call sites must stay bare `&str` — not `.to_string()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn strings_starts_with_literal_prefix_compiles() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "adapter.wj",
        r#"
use std::strings

pub fn has_bearer_prefix(value: string) -> bool {
    return strings.starts_with(value, "Bearer ")
}
"#,
    );
    test.add_file("mod.wj", "pub mod adapter");

    let map = test.compile().expect("compile");
    let rs = map.get("adapter.rs").expect("adapter.rs");
    assert!(
        rs.contains("starts_with(") && rs.contains("\"Bearer \""),
        "expected bare &str literal prefix. Got:\n{rs}"
    );
    assert!(
        !rs.contains("\"Bearer \".to_string()"),
        "must not coerce &str param to String. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}

#[test]
fn strings_contains_literal_substring_compiles() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "adapter.wj",
        r#"
use std::strings

pub fn mentions_foo(value: string) -> bool {
    return strings.contains(value, "foo")
}
"#,
    );
    test.add_file("mod.wj", "pub mod adapter");

    let map = test.compile().expect("compile");
    let rs = map.get("adapter.rs").expect("adapter.rs");
    assert!(
        !rs.contains("\"foo\".to_string()"),
        "contains substring literal must stay &str. Got:\n{rs}"
    );
    test.assert_compiles_without_error();
}
