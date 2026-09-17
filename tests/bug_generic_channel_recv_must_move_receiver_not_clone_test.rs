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

//! P3.332: `recv<T>(rx) -> (Receiver<T>, Option<T>)` must move `rx`, not `rx.clone()`.
//! Tip RED: emits `rx.clone()` on non-Clone mpsc::Receiver wrapper (blocks generic wj-sync).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/generic_channel_recv_move.wj");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read fixture {p:?}: {e}"))
}

fn bad_recv(rs: &str) -> bool {
    // Match recv body cloning the receiver handle.
    rs.lines().any(|l| l.contains("rx.clone()"))
}

#[test]
fn generic_channel_recv_must_move_receiver_not_clone() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), fixture()).unwrap();
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
    if bad_recv(&generated) {
        eprintln!("P3.332 RED (snippet):\n{}", &generated[..generated.len().min(2000)]);
    }
    assert!(
        !bad_recv(&generated),
        "RED P3.332: recv must move Receiver, not rx.clone():\n{}",
        generated
            .lines()
            .filter(|l| l.contains("recv") || l.contains("clone") || l.contains("rx"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
