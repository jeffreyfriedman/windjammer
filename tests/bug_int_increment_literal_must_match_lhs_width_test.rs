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

//! FAILING REPRO — under `--module-file`, int `i = i + 1` must not emit `1 as i32`.
//!
//! Product tip api-check (LedgerKit `domain/string_contains.wj` + postgres while loops):
//!   `i += 1 as i32` / `j + 1_i32` while indices are i64 → E0277/E0308.
//! Isolate single-file may GREEN; multipass `--module-file` matches product RED.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/int_increment_literal_must_match_lhs_width.wj");

fn bad_i32_increment(rs: &str) -> bool {
    rs.contains("+= 1 as i32")
        || rs.contains("+= 1_i32")
        || rs.contains("+ 1_i32")
        || rs.contains("+ 1i32")
        || rs.contains("1_i32) as usize")
}

#[test]
fn int_increment_literal_must_match_lhs_width() {
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
    assert!(
        build.status.success(),
        "wj --module-file build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let mut rs = String::new();
    for entry in fs::read_dir(&out).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            rs.push_str(&fs::read_to_string(&path).unwrap_or_default());
            rs.push('\n');
        }
    }

    let bad = bad_i32_increment(&rs);
    if bad {
        eprintln!("RED P3.267 int increment width under --module-file:\n{rs}");
    }
    assert!(
        !bad,
        "RED P3.267: module-file int increment must not emit i32 literal into i64. Generated:\n{rs}"
    );
}
