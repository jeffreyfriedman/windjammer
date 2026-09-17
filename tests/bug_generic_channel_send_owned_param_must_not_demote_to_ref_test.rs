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

//! P3.331: `send<T>(tx, value: T)` must emit owned `value: T` and `tx.send(value)`.
//! Tip RED: demotes to `value: &T` + `value.clone()` (blocks generic wj-sync channels).

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/generic_channel_send_owned.wj");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read fixture {p:?}: {e}"))
}

fn bad_send(rs: &str) -> bool {
    rs.contains("value: &T")
        || rs.contains("send(value.clone())")
        || rs.contains("fn send<T: Clone>")
}

#[test]
fn generic_channel_send_owned_param_must_not_demote_to_ref() {
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
    if bad_send(&generated) {
        eprintln!("P3.331 RED (snippet):\n{}", &generated[..generated.len().min(2000)]);
    }
    assert!(
        !bad_send(&generated),
        "RED P3.331: send<T> must keep owned value: T (not &T / clone):\n{}",
        generated
            .lines()
            .filter(|l| l.contains("fn send") || l.contains("send(value") || l.contains("value:"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    assert!(
        generated.contains("value: T") || generated.contains("value: T,"),
        "expected owned value: T in signature:\n{generated}"
    );
}
