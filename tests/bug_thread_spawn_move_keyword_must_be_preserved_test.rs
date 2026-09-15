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

//! FAILING REPRO — `std::thread::spawn(move \|\| …)` must preserve the `move` keyword.
//!
//! Ecosystem `wj-sync` shared-inbox Pool:
//! ```ignore
//! std::thread::spawn(move || { inbox.lock()… })
//! ```
//! Tip library multipass emits `spawn(|| {…})` (move stripped) → E0373 borrow of `inbox`.
//! P3.294 only asserted absence of `spawn(&(move`; stripping `move` still breaks Arc capture.

#[path = "common/test_utils.rs"]
mod test_utils;

const MOVE_PRESERVE: &str = r#"
use std::sync::mpsc
use std::sync::{Arc, Mutex}

struct Cell {
    n: int,
}

pub fn parallel_inc() -> int {
    let pair = mpsc::channel()
    let tx = pair.0
    let rx = pair.1
    let cell = Arc::new(Mutex::new(Cell { n: 0 }))
    std::thread::spawn(move || {
        match cell.lock() {
            Ok(mut g) => {
                g.n = g.n + 1
                match tx.send(g.n) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    })
    match rx.recv() {
        Ok(v) => v,
        Err(_) => 0,
    }
}
"#;

#[test]
fn thread_spawn_move_keyword_must_be_preserved() {
    let generated = test_utils::compile_single(MOVE_PRESERVE);
    let has_move = generated.contains("spawn(move ||")
        || generated.contains("spawn(move||")
        || generated.contains("spawn(move |");
    assert!(
        has_move && !generated.contains("spawn(&(move"),
        "spawn(move ||) must emit move by value (not strip move, not wrap &):\n{generated}"
    );
}

#[test]
fn thread_spawn_move_keyword_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(MOVE_PRESERVE, &["thread::spawn"]);
}
