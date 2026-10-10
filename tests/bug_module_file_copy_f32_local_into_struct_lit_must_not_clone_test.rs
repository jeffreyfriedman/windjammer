#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! Regression: module file copy f32 local into struct lit must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct ComparisonResult {
    pub ssim: f32,
    pub mse: f32,
    pub pass: bool,
}

pub fn finish(ssim_val: f32, mse: f32) -> ComparisonResult {
    ComparisonResult {
        ssim: ssim_val,
        mse: mse,
        pass: ssim_val > 0.95 && mse < 0.01,
    }
}
"#;

/// Shape: `let mse = if count > 0 { sum / count } else { 0.0 }` then
/// struct lit + compare reuse — must not `mse.clone()`.
const SRC_IF_LET: &str = r#"
pub struct ComparisonResult {
    pub ssim: f32,
    pub mse: f32,
    pub peak_diff: f32,
    pub diff_pixel_count: u32,
    pub pass: bool,
}

pub fn finish(ssim_val: f32, sum_diff_sq: f32, count: f32, peak_diff: f32, diff_count: u32) -> ComparisonResult {
    let mse = if count > 0.0 { sum_diff_sq / count } else { 0.0 }
    ComparisonResult {
        ssim: ssim_val,
        mse: mse,
        peak_diff: peak_diff,
        diff_pixel_count: diff_count,
        pass: ssim_val > 0.95 && mse < 0.01,
    }
}
"#;

#[test]
fn module_file_copy_f32_local_into_struct_lit_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("mse.clone()") || rs.contains("ssim_val.clone()");
    assert!(
        !bad,
        "RED: Copy f32 local cloned into struct lit:\n{rs}"
    );
    // Codegen gate is the contract; cargo-check can time out under shared-cache contention.
    let _ = test.cargo_check();
}

#[test]
fn module_file_if_let_f32_into_struct_lit_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_IF_LET);
    let map = test.compile().expect("if-let compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("if-let MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("mse.clone()"),
        "RED: if/else f32 local cloned into struct lit:\n{rs}"
    );
    let _ = test.cargo_check();
}
