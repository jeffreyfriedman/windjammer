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

//! P3.538: `let mut prev = -1` then `prev = m.version` (field `int`/`i64`) must not
//! emit `prev = -1_i32` (E0308). Product `wj-migrate` `validate_unique_versions`.
//! Distinct from P3.527 (`i == 0` usize peer).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
struct Migration {
    version: int,
    name: string,
}

fn validate_unique_versions(items: Vec<Migration>) -> Result<(), string> {
    let mut prev = -1
    for m in items {
        if prev >= 0 && m.version <= prev {
            return Err("duplicate or out-of-order version")
        }
        prev = m.version
    }
    Ok(())
}

pub fn check(items: Vec<Migration>) -> Result<(), string> {
    validate_unique_versions(items)
}
"#;

#[test]
fn migrate_prev_sentinel_must_unify_with_version_i64() {
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
        "P3.538 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.538 validate_unique_versions:\n{rs}");
    assert!(
        !rs.contains("prev = -1_i32"),
        "P3.538 RED: -1 sentinel assigned from version:i64 must not stay i32:\n{rs}"
    );
    assert!(
        rs.contains("prev = -1_i64") || rs.contains("let mut prev: i64 = -1"),
        "P3.538: expected prev sentinel as i64 peer of m.version:\n{rs}"
    );
}
