/// TDD: f32/f64 unification when locals are inferred from float casts (dogfooding: squad_tactics.wj).
///
/// Bug: `let survival_rate = (alive as f32) / (total as f32); survival_rate < 0.3` emitted `0.3_f64`.
/// Root cause: `infer_type_from_expression` had no `Cast` arm, so `var_types` never stored `survival_rate`
/// as f32 and float comparison did not constrain the literal.
///
/// Fix: Infer `Type::Custom("f32"|"f64")` from `expr as f32` / `as f64` for variable type tracking.

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_name = format!("float_unify_{}_{}", std::process::id(), unique_id);
    let test_file = temp_dir.join(format!("{}.wj", test_name));
    let output_dir = temp_dir.join(&test_name);
    let output_file = output_dir.join(format!("{}.rs", test_name));

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");
    std::fs::write(&test_file, source).expect("Failed to write test file");

    let wj_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let status = Command::new(&wj_path)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .status()
        .expect("Failed to execute wj compiler");

    assert!(status.success(), "Compilation failed");

    let rust_code = std::fs::read_to_string(&output_file).expect("Failed to read generated Rust");

    let _ = std::fs::remove_file(&test_file);
    let _ = std::fs::remove_dir_all(&output_dir);

    rust_code
}

#[test]
fn test_cast_div_f32_local_comparison_literal_is_f32() {
    let source = r#"
pub fn check(alive: i32, total: i32) -> bool {
    let survival_rate = (alive as f32) / (total as f32)
    survival_rate < 0.3
}
"#;
    let output = compile_and_get_rust(source);
    assert!(
        output.contains("0.3_f32"),
        "expected 0.3_f32 (squad_tactics pattern), got:\n{}",
        output
    );
    assert!(
        !output.contains("0.3_f64"),
        "must not emit 0.3_f64 against f32 survival_rate, got:\n{}",
        output
    );
}

#[test]
fn test_cast_f32_local_compare_to_literal_after_let() {
    let source = r#"
pub fn almost_one(n: i32) -> bool {
    let x = n as f32
    x < 1.0
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("1.0_f32"), "expected 1.0_f32, got:\n{}", output);
    assert!(
        !output.contains("1.0_f64"),
        "must not use 1.0_f64, got:\n{}",
        output
    );
}

#[test]
fn test_cast_f64_local_still_emits_f64_literal() {
    let source = r#"
pub fn big(n: i32) -> bool {
    let x = n as f64
    x > 0.5
}
"#;
    let output = compile_and_get_rust(source);
    assert!(output.contains("0.5_f64"), "expected 0.5_f64, got:\n{}", output);
}
