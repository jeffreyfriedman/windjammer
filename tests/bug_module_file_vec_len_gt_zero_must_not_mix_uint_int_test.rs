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

//! P3.322: `rows.len() > 0` must not mix uint (len) with int literal `0`.
//!
//! LedgerKit tip api-check: `Ok(rows.len() > 0)` / `roles.len() > 0` → expected uint, found int.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub fn has_rows(rows: Vec<string>) -> bool {
    rows.len() > 0
}

pub fn first_if_any(roles: Vec<string>) -> string {
    if roles.len() > 0 {
        return roles[0]
    }
    ""
}
"#;

fn bad_int_zero_vs_len(rs: &str) -> bool {
    // len() emits usize; `> 0` must not force i64/`0` as int into uint compare.
    rs.contains("> 0_i64")
        || rs.contains("> 0i64")
        || rs.contains("len() > 0)")
            && (rs.contains("as i64") || rs.contains("_i64"))
}

#[test]
fn module_file_vec_len_gt_zero_must_not_mix_uint_int() {
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
        "wj build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    if !check.status.success() || bad_int_zero_vs_len(&generated) {
        eprintln!("P3.322 RED:\n{generated}\n{err}");
    }
    assert!(
        check.status.success() && !bad_int_zero_vs_len(&generated),
        "RED P3.322: vec.len() > 0 must unify uint/int:\n{generated}\n{err}"
    );
}
