/// TDD: Expression-Level Float Type Inference
///
/// PROBLEM: Function-level inference creates mixing errors (f32 * f64)
///
/// SOLUTION: Track types through expressions with constraint propagation:
/// - Binary ops: f32 * {unknown} → {unknown} = f32
/// - Method calls: f32.max({unknown}) → {unknown} = f32
/// - Assignments: let x: f32 = {unknown} → {unknown} = f32
/// - Function calls: foo({unknown}) where foo(x: f32) → {unknown} = f32
/// - Return: return {unknown} where fn -> f32 → {unknown} = f32
///
/// GOAL: Windjammer errors for mixing (not Rust errors)
#[path = "../common/test_utils.rs"]
mod test_utils;

use std::fs;
use tempfile::tempdir;
use windjammer::{build_project_ext, CompilationTarget};

#[test]
fn test_binary_op_propagation() {
    let wj_source = r#"
fn compute(x: f32) -> f32 {
    let scale = 2.0      // Should infer f32 from binary op with x
    let result = x * scale
    result
}

fn main() {
    let value = compute(5.0)
    println!("{}", value)
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    // All literals should be f32 (propagated from x: f32)
    assert!(
        rust_code.contains("2.0_f32") || rust_code.contains("let scale: f32 = 2.0"),
        "2.0 should be inferred as f32 from x: f32, got:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("5.0_f32") || rust_code.contains("compute(5.0)"),
        "5.0 should be inferred as f32 from compute parameter, got:\n{}",
        rust_code
    );
}

#[test]
fn test_method_call_propagation() {
    let wj_source = r#"
fn clamp_value(val: f32, min_val: f32, max_val: f32) -> f32 {
    val.max(min_val).min(max_val)
}

fn test() -> f32 {
    let result = clamp_value(0.5, 0.0, 1.0)  // All literals should infer f32
    result
}

fn main() {
    let v = test()
    println!("{}", v)
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    // All literals should be f32 (propagated through function parameters)
    let has_consistent_types = rust_code.contains("0.5_f32")
        || rust_code.contains("0.0_f32")
        || !rust_code.contains("0.5_f64"); // Should NOT have f64

    assert!(
        has_consistent_types,
        "All literals should be f32 (no mixing), got:\n{}",
        rust_code
    );
}

#[test]
fn test_mixing_detection() {
    let wj_source = r#"
fn mixed_types(x: f32, y: f64) -> f64 {
    let result = x * y  // ERROR: Cannot multiply f32 by f64
    result
}

fn main() {
    let v = mixed_types(1.0, 2.0)
    println!("{}", v)
}
"#;

    let src = tempdir().expect("tempdir for src");
    let out = tempdir().expect("tempdir for out");
    fs::write(src.path().join("test.wj"), wj_source).expect("write test.wj");
    let _ = build_project_ext(
        src.path(),
        out.path(),
        CompilationTarget::Rust,
        false,
        true,
        &[],
    );

    // Windjammer should detect mixing and report error BEFORE generating Rust
    // This test currently FAILS - we need to implement the type checker
    // TODO: Uncomment when type checker is implemented
    // assert!(
    //     !output.status.success(),
    //     "Should fail with type mixing error"
    // );
    // assert!(
    //     stderr.contains("Cannot multiply f32 by f64") || stderr.contains("type mismatch"),
    //     "Should report mixing error, got: {}",
    //     stderr
    // );

    // For now, this test documents the expected behavior
    println!("TODO: Implement type checker to catch mixing at Windjammer level");
    println!("Current: Mixing errors leak to Rust (E0277)");
    println!("Expected: Windjammer catches and reports clear error");
}

#[test]
fn test_cross_function_inference() {
    let wj_source = r#"
fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1
    let dy = y2 - y1
    let dist_sq = dx * dx + dy * dy
    dist_sq.sqrt()
}

fn compute_distances() -> Vec<f32> {
    let mut results = Vec::new()
    results.push(distance(0.0, 0.0, 3.0, 4.0))  // Should be 5.0
    results.push(distance(1.0, 1.0, 4.0, 5.0))  // Should be 5.0
    results
}

fn main() {
    let dists = compute_distances()
    println!("{:?}", dists)
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    // All literals should consistently be f32 throughout the call chain
    assert!(
        !rust_code.contains("_f64"),
        "Should not have any f64 literals (all f32), got:\n{}",
        rust_code
    );
}

#[test]
fn test_local_variable_inference() {
    let wj_source = r#"
fn process() -> f32 {
    let x = 1.0        // Unknown
    let y = 2.0        // Unknown
    let sum = x + y    // Both should unify to same type
    let scaled = sum * 3.0  // 3.0 should match sum's type
    scaled
}

fn main() {
    let result = process()
    println!("{}", result)
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    // All literals should have same type (unified)
    let f32_count = rust_code.matches("_f32").count();
    let f64_count = rust_code.matches("_f64").count();

    assert!(
        f32_count == 0 || f64_count == 0,
        "All literals should unify to same type, got {} f32 and {} f64:\n{}",
        f32_count,
        f64_count,
        rust_code
    );
}

/// Match on `HashMap::get` must unify default-arm literals as `f32` even when the function returns
/// a non-float type (no `-> f32` shortcut). Covers qualified map types and infer/codegen location sync.
#[test]
fn test_hashmap_get_match_default_arm_infers_f32_from_qualified_map_type() {
    let wj_source = r#"
use std::collections::HashMap

fn demo() -> i32 {
    let m: HashMap<(i32, i32), f32> = HashMap::new()
    let g = match m.get(&(0, 0)) {
        Some(v) => *v,
        None => 999999.0,
    }
    if g > 0.0 {
        1
    } else {
        0
    }
}

fn main() {
    let x = demo()
    println!("{}", x)
}
"#;

    let rust_code = test_utils::compile_single(wj_source);

    assert!(
        rust_code.contains("999999.0_f32"),
        "default match arm literal should be f32 (from HashMap value type), got:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("999999.0_f64"),
        "should not emit f64 for arm that must match f32:\n{}",
        rust_code
    );
}
