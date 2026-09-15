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

//! FAILING REPRO — `spawn(move ||)` inside worker loop after `inbox.clone()` strips `move`.
//!
//! P3.295 simple top-level Arc spawn is tip GREEN; wj-sync Pool shape still RED:
//! ```ignore
//! while true {
//!     let inbox = shared.clone()
//!     std::thread::spawn(move || { … inbox.lock() … })
//! }
//! ```
//! Tip may log `Unexpected token … While` with prior token `Or` at `move ||`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::sync::mpsc
use std::sync::{Arc, Mutex}

struct Cell {
    n: int,
}

pub fn spawn_workers(n: int) -> int {
    let pair = mpsc::channel()
    let tx = pair.0
    let rx = pair.1
    let shared = Arc::new(Mutex::new(Cell { n: 0 }))
    let mut i = 0
    while i < n {
        let inbox = shared.clone()
        let out = tx.clone()
        std::thread::spawn(move || {
            match inbox.lock() {
                Ok(mut g) => {
                    g.n = g.n + 1
                    match out.send(g.n) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        })
        i = i + 1
    }
    match rx.recv() {
        Ok(v) => v,
        Err(_) => 0,
    }
}
"#;

#[test]
fn module_file_spawn_move_in_worker_loop_must_be_preserved() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let has_move = generated.contains("spawn(move ||")
        || generated.contains("spawn(move||")
        || generated.contains("spawn(move |");
    assert!(
        has_move && !generated.contains("spawn(&(move"),
        "RED P3.297: worker-loop spawn(move ||) must preserve move:\n{generated}"
    );
}
