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

//! FAILING REPRO — library multipass strips `move` from `spawn(move \|\| …)`.
//!
//! Isolate `compile_single` may preserve `move` (P3.295 isolate GREEN), but
//! `wj build --library --module-file` for `wj-sync` Pool emits `spawn(|| …)` → E0373.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn module_file_spawn_move_keyword_must_be_preserved() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("lib.wj"),
        r#"
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
"#,
    )
    .unwrap();

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
        "library build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let has_move = generated.contains("spawn(move ||")
        || generated.contains("spawn(move||")
        || generated.contains("spawn(move |");
    assert!(
        has_move && !generated.contains("spawn(&(move"),
        "library multipass must preserve spawn(move ||):\n{generated}"
    );
}
