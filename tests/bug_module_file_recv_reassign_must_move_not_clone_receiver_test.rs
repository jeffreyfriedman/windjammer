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

//! FAILING REPRO — reassign loop `recv_int(rx)` must MOVE, not `.clone()`.
//!
//! Ecosystem `wj-sync` `drain_int_until_sentinel`:
//! ```ignore
//! let got = recv_int(rx)
//! rx = got.0
//! ```
//! Tip multipass emits `recv_int(rx.clone())` but `mpsc::Receiver` wrappers
//! are not `Clone` → E0599. Blocks tip-green `wj-sync` (incl. SharedMap get).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::sync::mpsc

pub struct IntReceiver {
    rx: mpsc::Receiver<int>,
}

pub fn recv_int(rx: IntReceiver) -> (IntReceiver, int) {
    let out = match rx.rx.recv() {
        Ok(v) => v,
        Err(_) => 0,
    }
    (rx, out)
}

pub fn drain(mut rx: IntReceiver) -> int {
    let mut sum: int = 0
    while true {
        let got = recv_int(rx)
        rx = got.0
        let v = got.1
        if v < 0 {
            break
        }
        sum = sum + v
    }
    sum
}
"#;

#[test]
fn module_file_recv_reassign_must_move_not_clone_receiver() {
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
    assert!(
        !generated.contains("recv_int(rx.clone())")
            && !generated.contains("recv_int(rx . clone())"),
        "RED P3.306: recv_int(rx) reassign must move, not clone non-Clone Receiver:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (recv reassign must move):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
