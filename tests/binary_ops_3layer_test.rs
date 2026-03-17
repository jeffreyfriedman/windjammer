//! TDD: Binary Operations 3-Layer Ownership Migration
//!
//! Tests for generate_binary_operation migration to 3-layer system.
//! CRITICAL: Int/float logic preserved (both_int prevents casting in division).
//! Replaces ad-hoc XOR deref with systematic Copy semantics.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
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
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc_output {
        Ok(output) => {
            let rustc_success = output.status.success();
            let rustc_err = String::from_utf8_lossy(&output.stderr).to_string();
            (rustc_success, generated, rustc_err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

// =============================================================================
// Copy refs in binary ops - auto-copy, no explicit deref
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_add_copy_refs() {
    let src = r#"
pub fn add(a: &i32, b: &i32) -> i32 {
    a + b
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("a + b"),
        "&i32 in BinaryOp: auto-copy. Got:\n{}",
        result
    );
    assert!(
        !result.contains("*a + *b"),
        "Should not add explicit deref for Copy. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_sub_copy_refs() {
    let src = r#"
pub fn sub(a: &i32, b: &i32) -> i32 {
    a - b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a - b"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_mul_copy_refs() {
    let src = r#"
pub fn mul(a: &i32, b: &i32) -> i32 {
    a * b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a * b"));
}

// =============================================================================
// CRITICAL: Int division preserved (no float cast)
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_int_division_preserved() {
    let src = r#"
pub fn half(x: i32) -> i32 {
    x / 2
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("x / 2"),
        "Integer division must stay int. Got:\n{}",
        result
    );
    assert!(
        !result.contains("(2) as"),
        "CRITICAL: 2 must NOT be cast to float. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_int_mod_preserved() {
    let src = r#"
pub fn remainder(x: i32, y: i32) -> i32 {
    x % y
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("x % y"));
}

// =============================================================================
// Mixed int/float - explicit cast required (DESIGN DECISION)
// =============================================================================

// DESIGN DECISION: Windjammer requires explicit casts for numeric type mixing
// Why: Prevents precision loss (i64→f32), truncation (f32→i32), and signedness bugs
// Philosophy: "Be explicit about what matters" - type conversions matter
// Same as: Rust, Swift, Kotlin (industry standard for safe languages)
//
// To mix types, use explicit cast:
//   x: f32 + (y as f32)  // Clear intent, no hidden bugs
//
// Test kept to document this design decision.
#[test]
#[ignore] // Not a feature to implement - documents explicit cast requirement
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_mixed_int_float_cast() {
    let src = r#"
pub fn compute(x: f32, y: i32) -> f32 {
    x + y
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("(y) as f32") || result.contains("y as f32"),
        "Mixed: y should be cast to f32. Got:\n{}",
        result
    );
}

// DESIGN DECISION: Windjammer requires explicit casts for numeric type mixing
// (Same rationale as test_binary_mixed_int_float_cast - applies to literals too)
// To mix: x: f32 * 2.0  or  x * (2 as f32)  - be explicit.
#[test]
#[ignore] // Not a feature to implement - documents explicit cast requirement
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_float_plus_int_literal() {
    let src = r#"
pub fn scale(x: f32) -> f32 {
    x * 2
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(
        result.contains("as f32") || result.contains("2.0"),
        "Int literal with float should cast. Got:\n{}",
        result
    );
}

// =============================================================================
// Comparisons - auto-deref via 3-layer
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_comparison_auto_deref() {
    let src = r#"
pub fn compare(a: &i32, b: &i32) -> bool {
    a == b
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("a == b"),
        "Comparison: auto-deref. Got:\n{}",
        result
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_comparison_lt_gt() {
    let src = r#"
pub fn less(a: &i32, b: &i32) -> bool {
    a < b
}
pub fn greater(a: &i32, b: &i32) -> bool {
    a > b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a < b"));
    assert!(result.contains("a > b"));
}

// =============================================================================
// Nested binary ops - precedence preserved
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_nested_precedence() {
    let src = r#"
pub fn expr(a: &i32, b: &i32, c: &i32) -> i32 {
    a + b * c
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(
        result.contains("a + b * c") || result.contains("a + (b * c)"),
        "Precedence. Got:\n{}",
        result
    );
}

// =============================================================================
// Bitwise ops - 3-layer
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_bitwise_and() {
    let src = r#"
pub fn band(a: &i32, b: &i32) -> i32 {
    a & b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a & b"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_bitwise_or() {
    let src = r#"
pub fn bor(a: &i32, b: &i32) -> i32 {
    a | b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a | b"));
}

// =============================================================================
// Logical ops
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_logical_and() {
    let src = r#"
pub fn land(a: bool, b: bool) -> bool {
    a && b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a && b"));
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_logical_or() {
    let src = r#"
pub fn lor(a: bool, b: bool) -> bool {
    a || b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a || b"));
}

// =============================================================================
// Multiple borrowed params
// =============================================================================

// TODO: Fix complex reference handling in binary ops
// Feature: Multiple levels of references in expressions (&&&i32)
// Status: Edge case - needs investigation
#[test]
#[ignore]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_add_three_refs() {
    let src = r#"
pub fn add_three(a: &i32, b: &i32, c: &i32) -> i32 {
    a + b + c
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a + b + c"));
}

// =============================================================================
// Both int arithmetic - no float
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_both_int_arithmetic() {
    let src = r#"
pub fn calc(a: i32, b: i32) -> i32 {
    (a + b) / 2
}
"#;
    let (success, result, err) = compile_and_verify(src);
    assert!(success, "Must compile. Error:\n{}", err);
    assert!(
        result.contains("(a + b) / 2")
            || (result.contains("a + b") && result.contains("/ 2")),
        "Integer arithmetic. Got:\n{}",
        result
    );
    assert!(
        !result.contains("(2) as f32"),
        "2 must not be cast to float. Got:\n{}",
        result
    );
}

// =============================================================================
// Comparison with usize
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_len_comparison() {
    let src = r#"
pub fn has_items(items: &Vec<i32>) -> bool {
    items.len() > 0
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    // Accept either: len() > 0 (direct) or !.is_empty() (Clippy optimization)
    assert!(
        (result.contains("len()") && result.contains(">")) || result.contains("is_empty()"),
        "Expected len()>0 or is_empty(). Got:\n{}",
        result
    );
}

// =============================================================================
// Field access in binary op
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_field_access() {
    let src = r#"
pub struct Point { pub x: i32, pub y: i32 }
pub fn sum(p: &Point) -> i32 {
    p.x + p.y
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(
        result.contains("p.x") && result.contains("p.y") && result.contains("+"),
        "Field access in binary. Got:\n{}",
        result
    );
}

// =============================================================================
// Shift ops
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_shift_left() {
    let src = r#"
pub fn shl(x: i32, n: i32) -> i32 {
    x << n
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("<<"));
}

// =============================================================================
// XOR
// =============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_xor() {
    let src = r#"
pub fn xor(a: &i32, b: &i32) -> i32 {
    a ^ b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a ^ b"));
}

// =============================================================================
// Mixed owned and borrowed
// =============================================================================

// TODO: Fix mixed ownership handling in binary ops
// Feature: Owned + borrowed in binary ops (i32 + &i32)
// Status: Needs proper auto-deref logic
#[test]
#[ignore]
#[cfg_attr(tarpaulin, ignore)]
fn test_binary_mixed_owned_borrowed() {
    let src = r#"
pub fn mix(a: i32, b: &i32) -> i32 {
    a + b
}
"#;
    let (success, result, _) = compile_and_verify(src);
    assert!(success);
    assert!(result.contains("a + b"));
}
