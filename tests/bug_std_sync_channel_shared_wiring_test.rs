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

//! FAILING REPRO — idiomatic `std::sync` channel + Shared for `wj-sync` graduation.
//!
//! Runtime today re-exports Rust `mpsc` / Arc / Mutex helpers (`channel`, `mutex`).
//! Target vocabulary: `unbounded` / `send` / `recv` / `shared` (no Arc/Mutex in WJ).
//! Use `assert_stdlib_runtime_links` so substring-only checks cannot false-green.

#[path = "common/test_utils.rs"]
mod test_utils;

const CHANNEL: &str = r#"
use std::sync

pub fn ping() -> int {
    let pair = sync.unbounded()
    let _tx = sync.send(pair.0, 42)
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
    // Prefer WJ vocabulary needles once std/sync.wj exists; until then cargo check must fail.
    test_utils::assert_stdlib_runtime_links(
        CHANNEL,
        &[
            "windjammer_runtime::sync::unbounded",
            // interim: accept channel() only after std wraps it as unbounded
        ],
    );
}

#[test]
fn std_sync_shared_must_wire() {
    test_utils::assert_stdlib_runtime_links(
        SHARED,
        &["windjammer_runtime::sync::shared"],
    );
}
