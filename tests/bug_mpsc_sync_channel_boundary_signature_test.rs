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

//! FAILING REPRO — `std::sync::mpsc::sync_channel` needs a boundary signature.
//!
//! Ecosystem `wj-sync` bounded channels:
//! ```ignore
//! let pair = mpsc::sync_channel(2)
//! ```
//! Tip emits `compile_error!("missing boundary signature for mpsc::sync_channel")`.
//! Runtime already exposes `windjammer_runtime::sync::sync_channel`.

#[path = "common/test_utils.rs"]
mod test_utils;

const BOUNDED: &str = r#"
use std::sync::mpsc

pub fn bounded_roundtrip(v: int) -> int {
    let pair = mpsc::sync_channel(2)
    let tx = pair.0
    let rx = pair.1
    match tx.send(v) {
        Ok(_) => {}
        Err(_) => {
            return -1
        }
    }
    let out = match rx.recv() {
        Ok(x) => x,
        Err(_) => -1,
    }
    out
}
"#;

#[test]
fn mpsc_sync_channel_must_not_emit_missing_boundary_signature() {
    let generated = test_utils::compile_single(BOUNDED);
    assert!(
        !generated.contains("missing boundary signature for `mpsc::sync_channel`")
            && !generated.contains("missing boundary signature for mpsc::sync_channel"),
        "mpsc::sync_channel must have a boundary signature:\n{generated}"
    );
}

#[test]
fn mpsc_sync_channel_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(BOUNDED, &[]);
}
