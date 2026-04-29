/// TDD: No spurious `as f64` on f32 operands in float binary ops (E0308: f64 * f32).
///
/// When float inference says f32 on one side but `infer_expression_type` only knows `Type::Float`,
/// codegen must not treat that as f64 and promote the other operand.
use std::fs;
use std::process::Command;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

fn compile_single_file(source: &str) -> String {
    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");
    fs::write(src.path().join("test.wj"), source).expect("write test.wj");
    build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    )
    .expect("build_project_ext");
    let raw = fs::read_to_string(out.path().join("test.rs")).unwrap_or_default();
    raw.lines()
        .filter(|l| !l.contains("use super::"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_rustc(rs_code: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir for rustc");
    let rs_file = dir.path().join("test.rs");
    fs::write(&rs_file, rs_code).unwrap();

    let output = Command::new("rustc")
        .current_dir(dir.path())
        .arg("test.rs")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.success(), stderr)
}

#[test]
fn test_f32_acos_mul_float_literal_no_as_f64_on_left() {
    let source = r#"
pub fn angle_deg(value: f32) -> f32 {
    let x = value.acos() * 57.29
    x
}
"#;

    let output = compile_single_file(source);
    assert!(
        !output.contains("acos() as f64") && !output.contains(".acos() as f64"),
        "must not cast f32 acos() to f64 when multiplying by float literal in f32 context; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

#[test]
fn test_f32_literal_mul_subexpr_no_spurious_f64_cast() {
    let source = r#"
pub fn scaled(dist: f32) -> f32 {
    let y = 0.3 * (1.0 - dist)
    y
}
"#;

    let output = compile_single_file(source);
    assert!(
        !output.contains(" as f64"),
        "must not insert f64 promotion in f32 * (f32 - f32); got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}

#[test]
fn test_f32_field_mul_field_consistent_float() {
    let source = r#"
pub struct Vis { modifier: f32, visibility: f32 }

pub fn combine(v: Vis) -> f32 {
    let z = v.modifier * v.visibility
    z
}
"#;

    let output = compile_single_file(source);
    assert!(
        !output.contains(" as f64"),
        "f32 field * f32 field must not insert as f64; got:\n{}",
        output
    );

    let (ok, stderr) = run_rustc(&output);
    assert!(
        ok,
        "rustc failed:\nstderr: {}\n\nGenerated:\n{}",
        stderr, output
    );
}
