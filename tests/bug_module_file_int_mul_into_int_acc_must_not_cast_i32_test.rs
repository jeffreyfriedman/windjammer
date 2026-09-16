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

//! P3.317: `total = total + (n * mult)` with `n`/`mult`/`total: int` must not
//! emit `(n * mult) as i32` when `int` lowers to i64 (`wj-duration` parse_ms).
//!
//! Product: `Ok(total)` E0308 expected i64 found i32, or i64 += i32.
//! (P3.316 is WDB-235–238 coverage — do not collide.)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
pub fn accumulate(n: int, mult: int) -> Result<int, string> {
    let mut total = 0
    total = total + (n * mult)
    Ok(total)
}
"#;

fn bad_mul_as_i32(rs: &str) -> bool {
    rs.contains("(n * mult) as i32")
        || rs.contains("(n*mult) as i32")
        || rs.contains("* mult) as i32")
        || (rs.contains("as i32") && rs.contains("n * mult"))
}

#[test]
fn module_file_int_mul_into_int_acc_must_not_cast_i32() {
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
    if bad_mul_as_i32(&generated) {
        eprintln!("P3.317 RED:\n{generated}");
    }
    assert!(
        !bad_mul_as_i32(&generated),
        "RED P3.317: int*(int) into int accumulator must not cast to i32:\n{generated}"
    );

    let check = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(out.join("Cargo.toml"))
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "cargo check failed (duration accumulate shape):\n{}\n{}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
}
