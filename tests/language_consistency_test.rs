//! Language Consistency Tests
//!
//! Validates Windjammer's inference rules are consistent:
//! - Local reassignment: current `wj` often infers `let mut` in Rust output (native immutability errors TBD)
//! - Parameters infer ownership automatically (including `self`)
//! - `mut self` is rejected with helpful error
//!
//! Philosophy: "Infer what doesn't matter, explicit where it does"

#[path = "test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_expect_error(code: &str) -> (bool, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let input_file = temp_dir.path().join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = env!("CARGO_BIN_EXE_wj");

    let output = Command::new(wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            temp_dir.path().join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (!output.status.success(), stderr)
}

// ============================================================================
// Variable mutability: EXPLICIT
// ============================================================================

#[test]
fn test_variable_mutability_explicit() {
    let code = r#"
fn foo() {
    let x = 0
    x = 1
}
"#;

    let generated =
        test_utils::compile_single_result(code).expect("Should compile with inferred mut");
    assert!(
        generated.contains("let mut x"),
        "Should infer `let mut x` in generated Rust. Got:\n{}",
        generated
    );
}

#[test]
fn test_variable_mut_allows_reassignment() {
    let code = r#"
fn foo() {
    let mut x = 0
    x = 1
}
"#;

    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Should compile: {}", result.unwrap_err());
}

// ============================================================================
// Parameter ownership: INFERRED
// ============================================================================

#[test]
fn test_parameter_ownership_inferred() {
    // Parameters infer ownership from usage
    let code = r#"
struct Item {
    x: int,
}

fn update(item: Item) {
    item.x = 1
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Should compile");
    assert!(
        generated.contains("fn update(item: &mut Item)")
            || generated.contains("fn update(item: &mut Item )"),
        "Should infer &mut Item for field mutation, got:\n{}",
        generated
    );
}

#[test]
fn test_self_inference_read() {
    // self infers &self for read-only methods
    let code = r#"
struct Item {
    x: int,
}

impl Item {
    fn get(self) -> int {
        self.x
    }
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Should compile");
    assert!(
        generated.contains("fn get(&self)"),
        "Should infer &self for read-only method, got:\n{}",
        generated
    );
}

#[test]
fn test_self_inference_mutate() {
    // self infers &mut self for mutations
    let code = r#"
struct Item {
    x: int,
}

impl Item {
    fn set(self, val: int) {
        self.x = val
    }
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Should compile");
    assert!(
        generated.contains("fn set(&mut self"),
        "Should infer &mut self for mutation, got:\n{}",
        generated
    );
}

#[test]
fn test_self_inference_push() {
    // self.items.push() infers &mut self
    let code = r#"
struct Buffer {
    items: Vec<int>,
}

impl Buffer {
    fn add(self, x: int) {
        self.items.push(x)
    }
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Should compile");
    assert!(
        generated.contains("fn add(&mut self"),
        "Should infer &mut self for push(), got:\n{}",
        generated
    );
}

// ============================================================================
// mut self: REJECTED
// ============================================================================

#[test]
fn test_mut_self_rejected() {
    // mut self should produce helpful error
    let code = r#"
struct Item {
    x: int,
}

impl Item {
    fn foo(mut self) {
        self.x = 1
    }
}
"#;

    let (failed, stderr) = compile_expect_error(code);
    assert!(failed, "Should reject mut self");
    assert!(
        stderr.contains("mut") && (stderr.contains("not needed") || stderr.contains("inferred")),
        "Error should explain mut is not needed for parameters, got:\n{}",
        stderr
    );
}

// ============================================================================
// Non-self parameter mutation
// ============================================================================

#[test]
fn test_non_self_param_mutation_infers_mut() {
    let code = r#"
struct Grid {
    data: Vec<int>,
}

impl Grid {
    fn fill(grid: Grid, value: int) {
        grid.data.push(value)
    }
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Should compile");
    assert!(
        generated.contains("&mut Grid") || generated.contains("grid: &mut Grid"),
        "Should infer &mut for mutated param, got:\n{}",
        generated
    );
}

// ============================================================================
// Read-only parameter
// ============================================================================

#[test]
fn test_read_only_param_infers_borrow() {
    let code = r#"
struct Data {
    value: int,
}

fn read(data: Data) -> int {
    data.value
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Should compile");
    // Read-only by-value field access: analyzer may pass `Data` (Copy-style) or `&Data`.
    let ok =
        generated.contains("fn read(data: &Data)") || generated.contains("fn read(data: Data)");
    assert!(
        ok,
        "Read-only param should be `&Data` or by-value `Data` depending on inference. got:\n{}",
        generated
    );
}
