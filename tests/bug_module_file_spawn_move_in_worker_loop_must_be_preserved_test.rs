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

//! FAILING REPRO — library multipass strips `move` from `spawn(move \|\| …)`
//! when the closure body starts with `while` (wj-sync shared-inbox Pool).
//!
//! P3.295 simple Arc `spawn(move \|\| { match … })` is tip GREEN.
//! The Pool shape below emits bare `spawn(\|\| …)` → E0373.
//! Tip may log `Unexpected token … While` with prior token `Or` at `move \|\|`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Exact shape of `wj-sync` shared-inbox `pool_run_double` workers.
const POOL_WORKER_LOOP: &str = r#"
use std::sync::mpsc
use std::sync::{Arc, Mutex}

struct JobInbox {
    rx: mpsc::Receiver<int>,
}

pub fn pool_run(n: int) -> int {
    let jobs = mpsc::channel()
    let results = mpsc::channel()
    let job_tx = jobs.0
    let inbox = Arc::new(Mutex::new(JobInbox { rx: jobs.1 }))
    let res_tx = results.0
    let res_rx = results.1

    let mut started: int = 0
    while started < 2 {
        let inbox = inbox.clone()
        let tx = res_tx.clone()
        std::thread::spawn(move || {
            while true {
                let msg = match inbox.lock() {
                    Ok(g) => g.rx.recv(),
                    Err(_) => {
                        return
                    },
                }
                match msg {
                    Ok(v) => {
                        if v < 0 {
                            break
                        }
                        match tx.send(v * 2) {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(_) => {
                        break
                    }
                }
            }
        })
        started = started + 1
    }

    let mut i: int = 0
    while i < n {
        match job_tx.send(i) {
            Ok(_) => {}
            Err(_) => {}
        }
        i = i + 1
    }
    let mut s: int = 0
    while s < 2 {
        match job_tx.send(-1) {
            Ok(_) => {}
            Err(_) => {}
        }
        s = s + 1
    }
    let mut sum: int = 0
    let mut got: int = 0
    while got < n {
        match res_rx.recv() {
            Ok(v) => {
                sum = sum + v
                got = got + 1
            }
            Err(_) => {
                break
            }
        }
    }
    sum
}
"#;

#[test]
fn module_file_spawn_move_in_worker_loop_must_be_preserved() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), POOL_WORKER_LOOP).unwrap();

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
        "RED P3.297: Pool worker-loop spawn(move ||) must preserve move (not bare spawn(||)):\n{generated}"
    );
}
