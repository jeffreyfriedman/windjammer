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
))]

//! `std::sync::mpsc::Receiver` is not `Clone` in Rust. Structs that wrap it
//! must not auto-derive Clone. `Arc<Mutex<T>>` and `mpsc::Sender` *are* Clone
//! and must keep auto-derive.

#[path = "common/test_utils.rs"]
mod test_utils;

fn last_derive_before(rust: &str, byte_idx: usize) -> Option<&str> {
    let prev = &rust[..byte_idx];
    let dpos = prev.rfind("#[derive(")?;
    let rest = &prev[dpos..];
    let end = rest.find("]\n").or_else(|| rest.find(']'))?;
    Some(&rest[..=end])
}

const MPSC_RECEIVER_STRUCT: &str = r#"
use std::sync::mpsc

pub struct IntReceiver {
    rx: mpsc::Receiver<int>,
}

pub fn make() -> IntReceiver {
    let pair = mpsc::channel()
    IntReceiver { rx: pair.1 }
}
"#;

const ARC_MUTEX_STRUCT: &str = r#"
use std::sync::{Arc, Mutex}

struct Cell {
    n: int,
}

pub struct SharedInt {
    inner: Arc<Mutex<Cell>>,
}

pub fn shared_int(n: int) -> SharedInt {
    SharedInt {
        inner: Arc::new(Mutex::new(Cell { n: n })),
    }
}

pub fn bump(s: SharedInt) -> SharedInt {
    match s.inner.lock() {
        Ok(mut g) => {
            g.n = g.n + 1
        }
        Err(_) => {}
    }
    s
}
"#;

const MPSC_SENDER_STRUCT: &str = r#"
use std::sync::mpsc

pub struct IntSender {
    tx: mpsc::Sender<int>,
}

pub fn make_tx() -> IntSender {
    let pair = mpsc::channel()
    IntSender { tx: pair.0 }
}
"#;

#[test]
fn test_struct_with_mpsc_receiver_skips_debug_clone_derive() {
    let output = test_utils::compile_single(MPSC_RECEIVER_STRUCT);
    assert!(!output.is_empty(), "expected generated Rust");

    let needle = "pub struct IntReceiver";
    let idx = output
        .find(needle)
        .unwrap_or_else(|| panic!("missing {needle}"));
    if let Some(attr) = last_derive_before(&output, idx) {
        assert!(
            !(attr.contains("Debug") && attr.contains("Clone")),
            "IntReceiver must not #[derive(Debug, Clone, ...)]; found: {attr}\n\n{output}"
        );
    }
}

#[test]
fn test_struct_with_mpsc_receiver_cargo_check_succeeds() {
    test_utils::assert_stdlib_runtime_links(MPSC_RECEIVER_STRUCT, &[]);
}

#[test]
fn test_struct_with_arc_mutex_keeps_clone_derive() {
    let output = test_utils::compile_single(ARC_MUTEX_STRUCT);
    let needle = "pub struct SharedInt";
    let idx = output
        .find(needle)
        .unwrap_or_else(|| panic!("missing {needle}\n{output}"));
    let attr = last_derive_before(&output, idx)
        .unwrap_or_else(|| panic!("expected derive before SharedInt\n{output}"));
    assert!(
        attr.contains("Clone"),
        "SharedInt with Arc<Mutex<_>> must derive Clone; found: {attr}\n\n{output}"
    );
}

#[test]
fn test_struct_with_arc_mutex_cargo_check_succeeds() {
    test_utils::assert_stdlib_runtime_links(ARC_MUTEX_STRUCT, &[]);
}

#[test]
fn test_struct_with_mpsc_sender_keeps_clone_derive() {
    let output = test_utils::compile_single(MPSC_SENDER_STRUCT);
    let needle = "pub struct IntSender";
    let idx = output
        .find(needle)
        .unwrap_or_else(|| panic!("missing {needle}\n{output}"));
    let attr = last_derive_before(&output, idx)
        .unwrap_or_else(|| panic!("expected derive before IntSender\n{output}"));
    assert!(
        attr.contains("Clone"),
        "IntSender must derive Clone (mpsc::Sender is Clone); found: {attr}\n\n{output}"
    );
}
