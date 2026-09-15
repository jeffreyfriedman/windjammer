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

//! FAILING REPRO — `0 - int` must keep one integer width (no `0_i32 - i64`).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/int_zero_minus_must_keep_lhs_width.wj");

#[test]
fn int_zero_minus_must_keep_lhs_width() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("mod.wj"), SOURCE).unwrap();
    let out = tmp.path().join("build");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(build.status.success(), "build failed: {}", String::from_utf8_lossy(&build.stderr));
    let mut rs = String::new();
    for entry in fs::read_dir(&out).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            rs.push_str(&fs::read_to_string(&path).unwrap_or_default());
        }
    }
    let bad = rs.contains("0_i32 -") || rs.contains("0i32 -") || rs.contains("0 as i32 -");
    if bad {
        eprintln!("RED P3.267 zero-minus width:\n{rs}");
    }
    assert!(!bad, "RED: 0 - int must not emit 0_i32. Generated:\n{rs}");
}
