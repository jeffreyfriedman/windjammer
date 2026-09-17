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

//! FAILING REPRO — `std::sync` channel + Shared graduation target for `wj-sync`.
//!
//! Ecosystem package is the reference (generics-first). Std should grow:
//! `unbounded` / `send` / `recv` / `Shared` / `wait` vocabulary (not Arc/Mutex).

#[path = "common/test_utils.rs"]
mod test_utils;

const CHANNEL: &str = r#"
use std::sync

pub fn ping() -> int {
    let pair = sync.unbounded()
    let tx = sync.send(pair.0, 42)
    let got = sync.recv(pair.1)
    match got.1 {
        Some(v) => v,
        None => 0,
    }
}
"#;

const SHARED: &str = r#"
use std::sync

pub fn bump() -> int {
    let mut s = sync.shared(0)
    s = sync.shared_add(s, 1)
    sync.shared_get(s)
}
"#;

#[test]
fn std_sync_unbounded_channel_must_wire() {
    let generated = test_utils::compile_single(CHANNEL);
    assert!(
        !generated.contains("compile_error!")
            && (generated.contains("unbounded") || generated.contains("sync::")),
        "std::sync.unbounded/send/recv must wire for wj-sync graduation:\n{generated}"
    );
}

#[test]
fn std_sync_shared_must_wire() {
    let generated = test_utils::compile_single(SHARED);
    assert!(
        !generated.contains("compile_error!")
            && (generated.contains("shared") || generated.contains("sync::")),
        "std::sync.shared must wire for wj-sync graduation:\n{generated}"
    );
}
