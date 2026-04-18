//! TDD: `partial_eq_types` registry must match codegen so nested structs and enums
//! derive `PartialEq` when rustc needs it (fixes E0277 from missing trait impls).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify_rustc(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return (
            false,
            String::new(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        );
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path).unwrap_or_default();

    let rs_path = temp_dir.path().join("test.rs");
    fs::write(&rs_path, &generated).expect("Failed to write rs file");

    let rustc = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg("--edition=2021")
        .arg(&rs_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output()
        .expect("rustc failed");

    let err = String::from_utf8_lossy(&rustc.stderr).to_string();
    (rustc.status.success(), generated, err)
}

/// Inner has no `@auto`; codegen still derives PartialEq. Outer must compile with `==`.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_struct_partial_eq_registry() {
    let code = r#"
pub struct Inner {
    x: i32,
}

pub struct Outer {
    inner: Inner,
}

pub fn same() -> bool {
    let a = Outer { inner: Inner { x: 1 } };
    let b = Outer { inner: Inner { x: 1 } };
    a == b
}
"#;
    let (ok, rs, err) = compile_and_verify_rustc(code);
    assert!(
        rs.contains("PartialEq") && rs.contains("Inner") && rs.contains("Outer"),
        "Expected PartialEq on nested structs. Generated:\n{}",
        rs
    );
    assert!(
        !err.contains("E0277"),
        "Should not hit E0277 (PartialEq). rustc stderr:\n{}",
        err
    );
    assert!(ok, "rustc should accept generated Rust. stderr:\n{}", err);
}

/// Enum derives PartialEq only when payload types support it; custom struct must be registered.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_payload_struct_partial_eq_registry() {
    let code = r#"
pub struct Point {
    x: i32,
    y: i32,
}

pub enum Shape {
    Dot(Point),
    None,
}

pub fn same() -> bool {
    let a = Shape::Dot(Point { x: 0, y: 0 });
    let b = Shape::Dot(Point { x: 0, y: 0 });
    a == b
}
"#;
    let (ok, rs, err) = compile_and_verify_rustc(code);
    assert!(
        rs.contains("PartialEq") && rs.contains("Shape") && rs.contains("Point"),
        "Expected PartialEq on enum and struct. Generated:\n{}",
        rs
    );
    assert!(
        !err.contains("E0277"),
        "Should not hit E0277. stderr:\n{}",
        err
    );
    assert!(ok, "rustc should accept generated Rust. stderr:\n{}", err);
}
