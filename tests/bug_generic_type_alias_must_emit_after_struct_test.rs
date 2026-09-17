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

//! P3.333: `pub type SharedInt = Shared<int>` must appear **after** `struct Shared<T>`
//! in generated Rust. Tip RED: aliases are hoisted above the struct (E0425).
//! Blocks generics-first `wj-sync` Shared/Pending aliases.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn fixture() -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/generic_type_alias_after_struct.wj");
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read fixture {p:?}: {e}"))
}

fn alias_before_struct(rs: &str) -> bool {
    let alias = rs.find("type SharedInt");
    let strukt = rs.find("struct Shared");
    match (alias, strukt) {
        (Some(a), Some(s)) => a < s,
        _ => true,
    }
}

#[test]
fn generic_type_alias_must_emit_after_struct() {
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
    if alias_before_struct(&generated) {
        eprintln!("P3.333 RED (snippet):\n{}", &generated[..generated.len().min(1500)]);
    }
    assert!(
        !alias_before_struct(&generated),
        "RED P3.333: type alias SharedInt must appear after struct Shared:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed:\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
