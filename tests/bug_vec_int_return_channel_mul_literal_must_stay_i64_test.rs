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

//! `-> Vec<int>` must not demote `v * 2` to `v * 2_i32` (wj-sync `pool_run_double`).
//!
//! `function_prefers_i32_coord_locals` treated non-int returns as i32-coord builders;
//! `Vec<int>` fell through to that path and broke channel `i64` payloads.

#[path = "common/test_utils.rs"]
mod test_utils;

const POOL_DOUBLE: &str = r#"
use std::sync::mpsc

pub fn pool_run_double(n: int) -> Vec<int> {
    let results = mpsc::channel()
    let res_tx = results.0
    let res_rx = results.1
    let mut i: int = 0
    while i < n {
        let tx = res_tx.clone()
        let v = i
        std::thread::spawn(|| {
            match tx.send(v * 2) {
                Ok(_) => {}
                Err(_) => {}
            }
        })
        i = i + 1
    }
    let mut out: Vec<int> = Vec::new()
    let mut got: int = 0
    while got < n {
        match res_rx.recv() {
            Ok(v) => {
                out.push(v)
                got = got + 1
            }
            Err(_) => {
                break
            }
        }
    }
    out
}
"#;

#[test]
fn vec_int_return_channel_mul_literal_must_stay_i64() {
    let generated = test_utils::compile_single(POOL_DOUBLE);
    assert!(
        !generated.contains("2_i32") && !generated.contains("* 2 as i32"),
        "Vec<int> channel payload multiply must keep i64 literal peer:\n{generated}"
    );
    assert!(
        generated.contains("2_i64") || generated.contains("* 2)"),
        "expected i64-friendly multiply in generated Rust:\n{generated}"
    );
}

#[test]
fn vec_int_return_channel_mul_literal_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(POOL_DOUBLE, &["thread::spawn", "send"]);
}
