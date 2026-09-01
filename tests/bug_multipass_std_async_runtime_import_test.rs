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

//! FAILING REPRO — `use std::async_runtime::sleep_ms_blocking` must not emit
//! `use windjammer_runtime::async_runtime as async` (Rust keyword alias) and must
//! import `sleep_ms_blocking` into scope.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const PAUSE_ADAPTER: &str = include_str!("fixtures/stdlib/pause_ms_async_runtime.wj");

#[test]
fn multipass_std_async_runtime_import_must_not_alias_async_keyword() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod pause
"#,
    );
    test.add_file("pause.wj", PAUSE_ADAPTER);

    let map = test.compile().expect("pause adapter fixture should compile");
    let pause_rs = map.get("pause.rs").expect("pause.rs");

    assert!(
        !pause_rs.contains("async_runtime as async"),
        "RED: must not alias async_runtime as Rust keyword `async`; emitted:\n{pause_rs}"
    );
    assert!(
        pause_rs.contains("sleep_ms_blocking("),
        "RED: must call sleep_ms_blocking; emitted:\n{pause_rs}"
    );
    test.cargo_check()
        .expect("multipass std::async_runtime import fixture must cargo-check");
}
