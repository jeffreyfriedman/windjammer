/// TDD Tests: Compiler-side immutability enforcement (v0.41.0)
///
/// The Windjammer compiler should emit its own errors (not just rustc errors)
/// when `let` bindings are mutated. All mutation patterns should be caught:
/// - Direct reassignment: `let x = 5; x = 10`
/// - Compound assignment: `let count = 0; count += 1`
/// - Field mutation: `let point = Point { x: 0 }; point.x = 10`
/// - Mutating method call: `let items = Vec::new(); items.push(1)`
///
/// `let mut` bindings should allow all of these.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Compile a .wj file and return (exit_code, stdout, stderr)
fn compile_wj(source: &str) -> (i32, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, source).expect("Failed to write test file");

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo") // Don't run cargo, we just want Windjammer errors
        .output()
        .expect("Failed to execute compiler");

    let exit_code = result.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    (exit_code, stdout, stderr)
}

// ==========================================
// Direct reassignment errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_direct_reassignment_is_error() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let x = 5
    x = 10
}
"#,
    );
    assert_ne!(
        exit_code, 0,
        "Compiler should fail when reassigning immutable `let` binding"
    );
    assert!(
        stderr.contains("cannot assign")
            || stderr.contains("immutable")
            || stderr.contains("mutability"),
        "Error should mention immutability, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("let mut"),
        "Error should suggest `let mut`, got:\n{}",
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_direct_reassignment_is_ok() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let mut x = 5
    x = 10
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when reassigning `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Compound assignment errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_compound_assignment_is_error() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let count = 0
    count += 1
}
"#,
    );
    assert_ne!(
        exit_code, 0,
        "Compiler should fail when using compound assignment on immutable `let` binding"
    );
    assert!(
        stderr.contains("cannot use compound assignment")
            || stderr.contains("immutable")
            || stderr.contains("not declared as mutable"),
        "Error should mention immutability for compound assignment, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("let mut"),
        "Error should suggest `let mut`, got:\n{}",
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_compound_assignment_is_ok() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let mut count = 0
    count += 1
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when using compound assignment on `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Field mutation errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_field_mutation_is_error() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
struct Point { x: int, y: int }

fn main() {
    let point = Point { x: 0, y: 0 }
    point.x = 10
}
"#,
    );
    assert_ne!(
        exit_code, 0,
        "Compiler should fail when mutating field of immutable `let` binding"
    );
    assert!(
        stderr.contains("cannot mutate field")
            || stderr.contains("immutable")
            || stderr.contains("not declared as mutable"),
        "Error should mention field mutation of immutable binding, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("let mut"),
        "Error should suggest `let mut`, got:\n{}",
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_field_mutation_is_ok() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
struct Point { x: int, y: int }

fn main() {
    let mut point = Point { x: 0, y: 0 }
    point.x = 10
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when mutating field of `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Mutating method call errors
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mutating_method_call_is_error() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let items: Vec<int> = Vec::new()
    items.push(1)
}
"#,
    );
    assert_ne!(
        exit_code, 0,
        "Compiler should fail when calling mutating method on immutable `let` binding"
    );
    assert!(
        stderr.contains("cannot borrow")
            || stderr.contains("immutable")
            || stderr.contains("not declared as mutable"),
        "Error should mention mutating method on immutable binding, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("let mut"),
        "Error should suggest `let mut`, got:\n{}",
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_mutating_method_call_is_ok() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let mut items: Vec<int> = Vec::new()
    items.push(1)
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Should NOT error when calling mutating method on `let mut` binding, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Impl block coverage
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_immutability_enforced_in_impl_methods() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
struct Counter { value: int }

impl Counter {
    fn reset(self) {
        let x = 0
        x = 5
    }
}
"#,
    );
    assert_ne!(
        exit_code, 0,
        "Should detect immutability violations inside impl methods, stderr:\n{}",
        stderr
    );
}

// ==========================================
// Non-mutating operations should NOT error
// ==========================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_read_only_is_ok() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let x = 5
    let y = x + 1
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Read-only use of `let` binding should not error, stderr:\n{}",
        stderr
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_non_mutating_method_is_ok() {
    let (exit_code, _stdout, stderr) = compile_wj(
        r#"
fn main() {
    let items: Vec<int> = Vec::new()
    let n = items.len()
}
"#,
    );
    assert_eq!(
        exit_code, 0,
        "Non-mutating method on `let` binding should not error, stderr:\n{}",
        stderr
    );
}
