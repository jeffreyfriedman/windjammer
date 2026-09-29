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

//! P3.538: `let mut prev = -1` then `prev = m.version` when `version: int`
//! (emits `i64`) must not keep `prev` as `i32` (`prev = -1_i32` → E0308).
//!
//! Product `wj-migrate` `validate_unique_versions` (tip p3520 18:43).
//! P3.527 greened `i == 0_usize` for string-scan; this is **assignment unify**
//! of a negative sentinel with a struct `int` field. Do not reshape migrate.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
struct Migration {
    version: int,
    name: string,
}

pub fn validate_unique_versions(items: Vec<Migration>) -> Result<(), string> {
    let sorted = items
    let mut prev = -1
    let mut first = true
    for m in sorted {
        if !first {
            if m.version == prev {
                return Err("dup")
            }
        }
        prev = m.version
        first = false
    }
    Ok(())
}
"#;

fn prev_stays_i32(rs: &str) -> bool {
    rs.contains("let mut prev = -1_i32")
        || rs.contains("prev = -1_i32")
        || (rs.contains("prev: i32") && rs.contains("prev = m.version"))
}

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
    eprintln!("P3.538 validate_unique prev emit:\n{rs}");
    assert!(
        !prev_stays_i32(&rs),
        "P3.538 RED: prev sentinel must unify with Migration.version (i64), not stay i32:\n{rs}"
    );
    assert!(
        rs.contains("prev = m.version"),
        "P3.538: expected prev = m.version assignment:\n{rs}"
    );
}
