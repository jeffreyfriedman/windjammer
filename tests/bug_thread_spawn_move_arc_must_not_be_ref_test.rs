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

//! FAILING REPRO — `std::thread::spawn(move \|\| …)` with `Arc` capture still wraps `&(move \|\|…)`.
//!
//! Ecosystem `wj-sync` Pool shared-inbox workers need:
//! ```ignore
//! let inbox = Arc::new(Mutex::new(…))
//! std::thread::spawn(move || { inbox.lock()… })
//! ```
//! Tip emits `spawn(&(move || …))` → E0716. Plain `spawn(|| { tx.send(…) })` (P3.286) is green.

#[path = "common/test_utils.rs"]
mod test_utils;

const MOVE_ARC: &str = r#"
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
fn thread_spawn_move_arc_must_not_wrap_ref() {
    let generated = test_utils::compile_single(MOVE_ARC);
    assert!(
        !generated.contains("spawn(&(move")
            && !generated.contains("spawn(& (move"),
        "spawn(move ||) with Arc capture must pass closure by value:\n{generated}"
    );
}

#[test]
fn thread_spawn_move_arc_must_cargo_check() {
    test_utils::assert_stdlib_runtime_links(MOVE_ARC, &["thread::spawn"]);
}
