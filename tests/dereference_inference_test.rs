//! TDD Test: E0614 Over-Dereferencing Fix
//!
//! Error Pattern: type `f32` cannot be dereferenced (and i32, etc.)
//!
//! Root Cause: Compiler generates unnecessary `*` when value is already owned.
//! - *self.alert_level when alert_level is AlertLevel, not &AlertLevel
//! - *move_cost when move_cost is f32 (from tuple destructuring)
//! - *nx, *ny when these are i32 (from tuple destructuring)
//!
//! Key Principle: Only generate `*` when the value is actually a reference type
//! (&T or &mut T), never for owned values.
//!
//! Fix: In expression_generation.rs, only add dereference when the expression's
//! inferred type is actually Type::Reference or Type::MutableReference.

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
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
        return (stderr, false);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            temp_dir.path().join("test.rlib").to_str().unwrap(),
        ])
        .arg(&generated_path)
        .output();

    let compiles = rustc_output
        .as_ref()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !compiles {
        if let Ok(ref out) = rustc_output {
            eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&out.stderr));
        }
    }

    (generated, compiles)
}

// === Pattern 1: Tuple destructuring from Vec index (astar_grid pattern) ===

#[test]
fn test_tuple_destructure_binary_op_no_deref() {
    // let (nx, ny, move_cost) = neighbors[ni]
    // let tentative_g = current_g + move_cost  <- NO *move_cost
    let source = r#"
pub fn get_neighbors() -> Vec<(i32, i32, f32)> {
    vec![(0, 0, 1.0)]
}

pub fn pathfind() -> f32 {
    let neighbors = get_neighbors()
    let current_g = 0.0
    let (nx, ny, move_cost) = neighbors[0]
    let tentative_g = current_g + move_cost
    tentative_g
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        !result.contains("*move_cost"),
        "Should NOT add * to move_cost (f32 owned). Got:\n{}",
        result
    );
    assert!(
        result.contains("current_g + move_cost"),
        "Expected current_g + move_cost. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_in_hashmap_key_no_deref() {
    // g_score.get(&(nx, ny)) <- NO (*nx, *ny)
    let source = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<(i32, i32), f32>, neighbors: Vec<(i32, i32, f32)>) -> f32 {
    let (nx, ny, move_cost) = neighbors[0]
    match map.get(&(nx, ny)) {
        Some(v) => *v,
        None => move_cost
    }
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        !result.contains("(*nx, *ny)"),
        "Should NOT use (*nx, *ny) for owned tuple elements. Got:\n{}",
        result
    );
}

#[test]
fn test_tuple_destructure_local_var_no_deref() {
    // tentative_g, f are f32 - no *tentative_g or *f
    let source = r#"
pub fn pathfind() -> f32 {
    let neighbors = vec![(0, 0, 1.0)]
    let (nx, ny, move_cost) = neighbors[0]
    let tentative_g = 0.0 + move_cost
    let h = 1.0
    let f = tentative_g + h
    f
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        !result.contains("*tentative_g"),
        "Should NOT add * to tentative_g (f32). Got:\n{}",
        result
    );
    assert!(
        !result.contains("*f"),
        "Should NOT add * to f (f32). Got:\n{}",
        result
    );
}

// === Pattern 2: Struct field (self.alert_level) ===

#[test]
fn test_struct_field_owned_no_deref() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct AlertLevel {
    pub value: i32,
}

@derive(Debug, Clone, Copy)
pub struct State {
    alert_level: AlertLevel,
}

impl State {
    pub fn check(self) -> i32 {
        self.alert_level.value
    }
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        !result.contains("*self.alert_level"),
        "Should NOT dereference owned struct field. Got:\n{}",
        result
    );
}

// === Pattern 3: While loop with tuple destructuring (full astar pattern) ===

#[test]
fn test_while_loop_tuple_destructure_arithmetic() {
    let source = r#"
pub fn get_neighbors() -> Vec<(i32, i32, f32)> {
    vec![(0, 0, 1.0), (1, 0, 1.0)]
}

pub fn sum_path() -> f32 {
    let neighbors = get_neighbors()
    let mut total = 0.0
    let mut i = 0
    while i < neighbors.len() {
        let (nx, ny, move_cost) = neighbors[i]
        total = total + move_cost
        i = i + 1
    }
    total
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        !result.contains("*move_cost"),
        "Should NOT add * to move_cost in loop. Got:\n{}",
        result
    );
    // Accept both forms: total + move_cost or total += move_cost
    assert!(
        result.contains("total + move_cost") || result.contains("total += move_cost"),
        "Expected total + move_cost or total += move_cost. Got:\n{}",
        result
    );
}

// === Pattern 4: i32 variables in arithmetic ===

#[test]
fn test_i32_vars_no_deref_in_arithmetic() {
    let source = r#"
pub fn add_coords(items: Vec<(i32, i32)>) -> i32 {
    let (x, y) = items[0]
    x + y
}
"#;
    let (result, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", result);
    assert!(
        !result.contains("*x") && !result.contains("*y"),
        "Should NOT add * to i32 vars. Got:\n{}",
        result
    );
}
