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

//! P3.342: assigning `send(...)` / `shared_int(...)` must not inject
//! `let mut x: Sender<T>` (unbound T) or `Shared<i64>` without imports.
//! Blocks `wj test` for generics-first wj-sync even when the library cargo-checks.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/generic_assign_must_not_inject_unbound_t.wj");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read fixture {p:?}: {e}"))
}

fn bad_annotation(rs: &str) -> bool {
    // Struct/fn signatures legitimately use `Sender<T>`; only reject unbound
    // ascriptions on let bindings (P3.342).
    rs.lines().any(|line| {
        let t = line.trim_start();
        (t.starts_with("let ") || t.starts_with("let mut "))
            && (t.contains(": Sender<T>")
                || t.contains(": BoundedSender<T>")
                || (t.contains(": Pending<") && rs.contains("parallel_int")))
    })
}

#[test]
fn generic_assign_must_not_inject_unbound_sender_t() {
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
    // Also check any sibling if multipass splits — primary is lib.rs for single file.
    if bad_annotation(&generated) {
        eprintln!("P3.342 RED (snippet):\n{}", &generated[..generated.len().min(2000)]);
    }
    assert!(
        !bad_annotation(&generated),
        "RED P3.342: must not inject unbound Sender<T> on assign:\n{generated}"
    );
}
