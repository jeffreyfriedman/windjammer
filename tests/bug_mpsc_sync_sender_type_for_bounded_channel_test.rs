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

//! FAILING REPRO — `mpsc::sync_channel` returns `SyncSender`, not `Sender`.
//!
//! Ecosystem `wj-sync` bounded channels need a distinct sender type (or a unified
//! channel abstraction). Tip previously lacked a boundary signature (P3.287);
//! with signature present, wrapping `pair.0` in `Sender`-shaped structs fails:
//! `expected Sender<i64>, found SyncSender<_>`.

#[path = "common/test_utils.rs"]
mod test_utils;

const BOUNDED_WRAP: &str = r#"
use std::sync::mpsc

pub struct IntSender {
    tx: mpsc::Sender<int>,
}

pub struct IntReceiver {
    rx: mpsc::Receiver<int>,
}

pub fn bounded_int(cap: int) -> (IntSender, IntReceiver) {
    let pair = mpsc::sync_channel(cap)
    (
        IntSender { tx: pair.0 },
        IntReceiver { rx: pair.1 },
    )
}
"#;

const BOUNDED_SYNC_SENDER: &str = r#"
use std::sync::mpsc

pub struct BoundedIntSender {
    tx: mpsc::SyncSender<int>,
}

pub struct IntReceiver {
    rx: mpsc::Receiver<int>,
}

pub fn bounded_int(cap: int) -> (BoundedIntSender, IntReceiver) {
    let pair = mpsc::sync_channel(cap)
    (
        BoundedIntSender { tx: pair.0 },
        IntReceiver { rx: pair.1 },
    )
}

pub fn send_bounded(tx: BoundedIntSender, value: int) -> BoundedIntSender {
    match tx.tx.send(value) {
        Ok(_) => tx,
        Err(_) => tx,
    }
}
"#;

#[test]
fn sync_channel_must_not_wrap_as_plain_sender() {
    let generated = test_utils::compile_single(BOUNDED_WRAP);
    // Either missing signature (old) or wrong sender type — both are RED until
    // SyncSender is a first-class WJ channel surface.
    let missing_sig = generated.contains("missing boundary signature");
    let looks_ok = generated.contains("sync_channel")
        && !generated.contains("compile_error!")
        && generated.contains("SyncSender");
    assert!(
        missing_sig || !looks_ok,
        "bounded wrap must not silently type-check Sender←SyncSender:\n{generated}"
    );
}

#[test]
fn sync_sender_struct_field_must_cargo_check() {
    // Preferred idiomatic shape once SyncSender is wired as a type.
    test_utils::assert_stdlib_runtime_links(BOUNDED_SYNC_SENDER, &["sync_channel"]);
}
