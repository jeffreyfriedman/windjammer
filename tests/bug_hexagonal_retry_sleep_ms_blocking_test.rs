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

//! FAILING REPRO — hexagonal `adapters/clock` + `use std::async_runtime::sleep_ms_blocking`
//! must not emit `async_runtime as async` (`wj-retry` `pause_ms`). Same-module fixture is GREEN.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const PAUSE: &str = include_str!("fixtures/library_multipass/retry_pause_sleep_ms.wj");

#[test]
fn hexagonal_retry_sleep_ms_blocking_owned_int_must_cargo_check() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod backoff\n");
    project.add_file(
        "domain/backoff.wj",
        r#"
pub fn should_retry(attempt: int, max_attempts: int) -> bool {
    attempt < max_attempts
}
"#,
    );
    project.add_file("adapters/mod.wj", "pub mod clock\n");
    project.add_file("adapters/clock.wj", PAUSE);

    let map = project
        .compile()
        .expect("hexagonal retry clock compile should succeed");
    let clock_key = map
        .keys()
        .find(|k| k.ends_with("clock.rs"))
        .cloned()
        .unwrap_or_else(|| panic!("missing clock.rs; keys: {:?}", map.keys().collect::<Vec<_>>()));
    let clock_rs = map.get(&clock_key).expect("clock.rs");

    assert!(
        !clock_rs.contains("async_runtime as async"),
        "RED: hexagonal adapters/clock must not alias async_runtime as `async`; emitted:\n{clock_rs}"
    );
    assert!(
        clock_rs.contains("sleep_ms_blocking("),
        "must call sleep_ms_blocking; emitted:\n{clock_rs}"
    );
}
