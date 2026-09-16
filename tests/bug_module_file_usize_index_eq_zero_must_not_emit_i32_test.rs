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

//! P3.314: `while i < errors.len()` + `errors[i]` + `if i == 0` must not emit
//! `if i == 0_i32` when `i` is usize (index/len promotion).
//!
//! Product: `wj-validate` `all_ok` → E0308 / E0277 (`usize == i32`).
//! Related: P3.311 is the increment width (`i += 1 as i32`); this gate is the
//! compare-to-zero literal width.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub fn all_ok(errors: Vec<string>) -> Result<(), string> {
    if errors.len() == 0 {
        return Ok(())
    }
    let mut out = ""
    let mut i = 0
    while i < errors.len() {
        if i == 0 {
            out = errors[i]
        } else {
            out = "${out}; ${errors[i]}"
        }
        i = i + 1
    }
    Err(out)
}
"#;

fn bad_eq_zero_i32(rs: &str) -> bool {
    rs.contains("i == 0_i32")
        || rs.contains("i == 0i32")
        || (rs.contains("== 0_i32") && rs.contains("usize"))
}

#[test]
fn module_file_usize_index_eq_zero_must_not_emit_i32() {
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
        "library build failed:\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let generated = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    if bad_eq_zero_i32(&generated) {
        eprintln!("P3.314 RED:\n{generated}");
    }
    assert!(
        !bad_eq_zero_i32(&generated),
        "RED P3.314: usize index `i == 0` must not emit `0_i32`:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (validate-shaped index eq zero):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
