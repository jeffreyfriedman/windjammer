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

//! wj-sync: WJ `int` (i64) contexts must not emit `_i32` literal peers.
//! Covers call formals, compare/mul, tuple None arms, and `AtomicI64::new(0)`.

#[path = "common/test_utils.rs"]
mod test_utils;

const SHARED_ADD: &str = r#"
pub struct SharedInt {
    inner: int,
}

pub fn shared_int_add(s: SharedInt, delta: int) -> SharedInt {
    SharedInt {
        inner: s.inner + delta,
    }
}

pub fn call_add() {
    let s = SharedInt { inner: 0 }
    let _ = shared_int_add(s, 1)
}
"#;

const COMPARE_MUL: &str = r#"
pub fn check(v: int) -> bool {
    if v < 0 {
        return false
    }
    let _ = v * 2
    true
}
"#;

const MAP_NONE: &str = r#"
use std::collections::HashMap

pub fn lookup(m: HashMap<string, int>, key: string) -> (bool, int) {
    match m.get(key) {
        Some(v) => (true, *v),
        None => (false, 0),
    }
}
"#;


const VOID_BENCH_MUL: &str = r#"
pub fn bench_channel_peer() {
    let n: int = 100_000
    let _ = n * (n - 1)
    let _ = n * (n - 1) / 2
}
"#;

const ATOMIC_NEW: &str = r#"
use std::sync::atomic::AtomicI64

pub fn counter_new(n: int) -> AtomicI64 {
    AtomicI64::new(n)
}

pub fn zero_counter() {
    let _ = counter_new(0)
}
"#;

const ATOMIC_VOID_MAIN: &str = r#"
use std::sync::atomic::{AtomicI64, Ordering}

fn main() {
    let a = AtomicI64::new(0)
    a.fetch_add(1, Ordering::Relaxed)
    let _ = a.load(Ordering::Relaxed)
}
"#;


#[test]
fn wj_sync_void_bench_mul_literals_must_emit_i64() {
    let generated = test_utils::compile_single(VOID_BENCH_MUL);
    assert!(
        !generated.contains("1_i32") && !generated.contains("2_i32"),
        "void @test-style bench arith must not demote int literal peers to _i32:\n{generated}"
    );
    assert!(
        generated.contains("1_i64") || generated.contains("(n - 1)"),
        "expected i64 literal peer for n - 1:\n{generated}"
    );
}

#[test]
fn wj_sync_call_int_formal_literal_must_emit_i64() {
    let generated = test_utils::compile_single(SHARED_ADD);
    assert!(
        !generated.contains("1_i32"),
        "int formal call arg must not use 1_i32:\n{generated}"
    );
    assert!(
        generated.contains("1_i64") || generated.contains("shared_int_add(s, 1)"),
        "expected i64-width literal for delta:\n{generated}"
    );
}

#[test]
fn wj_sync_int_compare_and_mul_literals_must_emit_i64() {
    let generated = test_utils::compile_single(COMPARE_MUL);
    assert!(
        !generated.contains("0_i32") && !generated.contains("2_i32"),
        "int compare/mul peers must not use _i32:\n{generated}"
    );
    assert!(
        (generated.contains("0_i64") || generated.contains("< 0"))
            && (generated.contains("2_i64") || generated.contains("* 2)")),
        "expected i64-width compare/mul literals:\n{generated}"
    );
}

#[test]
fn wj_sync_tuple_none_zero_must_emit_i64() {
    let generated = test_utils::compile_single(MAP_NONE);
    assert!(
        !generated.contains("0_i32"),
        "None arm zero for int must not be 0_i32:\n{generated}"
    );
    assert!(
        generated.contains("0_i64") || generated.contains("(false, 0)"),
        "expected i64-width None arm zero:\n{generated}"
    );
}

#[test]
fn wj_sync_atomic_i64_new_zero_must_emit_i64() {
    let generated = test_utils::compile_single(ATOMIC_NEW);
    assert!(
        !generated.contains("0_i32"),
        "AtomicI64::new(0) must not use 0_i32:\n{generated}"
    );
    assert!(
        generated.contains("0_i64") || generated.contains("AtomicI64::new(0)"),
        "expected 0_i64 for AtomicI64::new:\n{generated}"
    );
}

#[test]
fn wj_sync_atomic_i64_void_main_literals_must_emit_i64() {
    let generated = test_utils::compile_single(ATOMIC_VOID_MAIN);
    assert!(
        !generated.contains("0_i32") && !generated.contains("1_i32"),
        "void main AtomicI64::new/fetch_add must not use _i32:\n{generated}"
    );
    assert!(
        generated.contains("0_i64") && (generated.contains("1_i64") || generated.contains("fetch_add(1,")),
        "expected i64-width AtomicI64 literals in void main:\n{generated}"
    );
}

#[test]
fn wj_sync_int_literal_peers_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(ATOMIC_NEW, &["AtomicI64", "new"]);
    test_utils::assert_stdlib_runtime_links(ATOMIC_VOID_MAIN, &["AtomicI64", "fetch_add"]);
}
