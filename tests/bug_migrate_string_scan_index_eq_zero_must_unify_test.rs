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

//! P3.527: after `while i < strings.len(stem)` the index is `usize`, but
//! `if i == 0` emits `i == 0_i64` (E0308 / E0277).
//!
//! Distinct from P3.516 (`while k <= vec.len()`). Product `wj-migrate`
//! `split_version_name`. Isolates match. Do not reshape the package.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

fn split_version_name(stem: string) -> Result<(int, string), string> {
    let mut i = 0
    while i < strings.len(stem) {
        let ch = strings.substring(stem, i, i + 1)
        if ch != "0" && ch != "1" && ch != "2" && ch != "3" && ch != "4" && ch != "5" && ch != "6" && ch != "7" && ch != "8" && ch != "9" {
            break
        }
        i = i + 1
    }
    if i == 0 {
        return Err("expected numeric version prefix")
    }
    if i >= strings.len(stem) || strings.substring(stem, i, i + 1) != "_" {
        return Err("expected underscore after version")
    }
    let version_text = strings.substring(stem, 0, i)
    let name = strings.substring(stem, i + 1, strings.len(stem))
    if strings.len(name) == 0 {
        return Err("empty migration name")
    }
    Ok((strings.parse_i32(version_text) as int, name))
}

pub fn parse_stem(stem: string) -> Result<(int, string), string> {
    split_version_name(stem)
}
"#;

#[test]
fn string_scan_index_eq_zero_must_unify() {
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
        "P3.527 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.527 split_version_name:\n{rs}");
    assert!(
        !rs.contains("i == 0_i64"),
        "P3.527 RED: usize scan index must not compare to 0_i64:\n{rs}"
    );
}
