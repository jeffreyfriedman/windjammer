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

//! FAILING REPRO — `std::thread::spawn(|| { … })` must pass the closure by value.
#[path = "common/test_utils.rs"]
mod test_utils;

const PARALLEL_ADD: &str = r#"
use std::sync::mpsc

pub struct PendingInt {
    rx: mpsc::Receiver<int>,
}

pub fn parallel_add(a: int, b: int) -> PendingInt {
    let pair = mpsc::channel()
    let tx = pair.0
    let rx = pair.1
    std::thread::spawn(|| {
        match tx.send(a + b) {
            Ok(_) => {}
            Err(_) => {}
        }
    })
    PendingInt { rx: rx }
}

pub fn wait_int(pending: PendingInt) -> int {
    let out = match pending.rx.recv() {
        Ok(v) => v,
        Err(_) => 0,
    }
    out
}
"#;

#[test]
fn thread_spawn_closure_must_not_be_passed_by_shared_ref() {
    let generated = test_utils::compile_single(PARALLEL_ADD);
    assert!(
        !generated.contains("spawn(&(") && !generated.contains("spawn(&move"),
        "std::thread::spawn must take the closure by value, not `&(move || …)`:\n{generated}"
    );
}

#[test]
fn thread_spawn_parallel_add_must_cargo_check() {
    // Expected GREEN once spawn codegen is fixed; RED today (E0716 / E0525).
    test_utils::assert_stdlib_runtime_links(PARALLEL_ADD, &[]);
}
