/// Tests for `let` immutability-by-default semantics
///
/// THE WINDJAMMER PHILOSOPHY:
/// - `let x = ...` is immutable - cannot be reassigned or mutated
/// - `let mut x = ...` is mutable - can be reassigned and mutated
/// - The compiler no longer silently infers `mut` for `let` bindings
///
/// This follows the modern language consensus (Rust, Swift, Kotlin, Zig):
/// Immutability by default makes code safer and intent clearer.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj(source: &str) -> (String, String) {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let generated_file = temp_dir.path().join("build").join("test.rs");
    let generated = fs::read_to_string(&generated_file).unwrap_or_else(|_| {
        panic!(
            "Failed to read generated file. Compiler output:\n{}",
            String::from_utf8_lossy(&wj_output.stderr)
        )
    });

    let stderr = String::from_utf8_lossy(&wj_output.stderr).to_string();
    (generated, stderr)
}

// ============================================================================
// TEST 1: `let` generates `let` (no mut) in Rust output
//
// A bare `let` binding that is never mutated should generate plain `let` in Rust.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_generates_immutable_binding() {
    let source = r#"
fn main() {
    let x = 5
    let y = x + 10
    println("{}", y)
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Should generate `let x = 5` (not `let mut x`)
    assert!(
        generated.contains("let x =") || generated.contains("let x:"),
        "Expected immutable `let x`, got:\n{}",
        generated
    );
    assert!(
        !generated.contains("let mut x"),
        "Should NOT have `let mut x` for immutable binding, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 2: `let mut` generates `let mut` in Rust output
//
// An explicit `let mut` binding should generate `let mut` in Rust.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_generates_mutable_binding() {
    let source = r#"
fn main() {
    let mut count = 0
    count = count + 1
    println("{}", count)
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Should generate `let mut count`
    assert!(
        generated.contains("let mut count"),
        "Expected `let mut count`, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 3: `let` without `mut` is REJECTED when mutated
//
// Previously, the compiler would silently add `mut` if the variable was mutated.
// Now, the compiler emits a Windjammer-native error before Rust codegen.
//
// This is the KEY BEHAVIORAL CHANGE: immutability is enforced at the
// Windjammer compiler level, not deferred to rustc.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_does_not_auto_infer_mut() {
    let source = r#"
fn main() {
    let items: Vec<int> = Vec::new()
    items.push(42)
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let stderr = String::from_utf8_lossy(&wj_output.stderr).to_string();

    // Compiler should REJECT this code with a mutability error
    assert!(
        !wj_output.status.success(),
        "Compiler should reject mutation of immutable `let` binding"
    );
    assert!(
        stderr.contains("not declared as mutable") || stderr.contains("immutable"),
        "Error should mention immutability. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("let mut"),
        "Error should suggest `let mut`. Got:\n{}",
        stderr
    );
}

// ============================================================================
// TEST 4: `let mut` with Vec operations works correctly
//
// When the user explicitly writes `let mut`, everything works as before.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_with_vec_push() {
    let source = r#"
fn main() {
    let mut items: Vec<int> = Vec::new()
    items.push(42)
    items.push(100)
    println("{}", items.len())
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // Should generate `let mut items`
    assert!(
        generated.contains("let mut items"),
        "Expected `let mut items` for explicit mut binding, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 5: `let` with compound assignment is REJECTED
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_compound_assignment_no_auto_mut() {
    let source = r#"
fn main() {
    let count = 0
    count += 1
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    fs::write(&test_file, source).unwrap();

    let wj_output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj compiler");

    let stderr = String::from_utf8_lossy(&wj_output.stderr).to_string();

    // Compiler should REJECT compound assignment on immutable binding
    assert!(
        !wj_output.status.success(),
        "Compiler should reject compound assignment on immutable `let` binding"
    );
    assert!(
        stderr.contains("compound assignment") || stderr.contains("immutable"),
        "Error should mention compound assignment on immutable binding. Got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("let mut"),
        "Error should suggest `let mut`. Got:\n{}",
        stderr
    );
}

// ============================================================================
// TEST 6: `let mut` with compound assignment works
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_mut_compound_assignment() {
    let source = r#"
fn main() {
    let mut count = 0
    count += 1
    println("{}", count)
}
"#;

    let (generated, _stderr) = compile_wj(source);

    assert!(
        generated.contains("let mut count"),
        "Expected `let mut count`, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 7: `let` and `let mut` coexist in same function
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_let_and_let_mut_coexist() {
    let source = r#"
fn main() {
    let name = "hello"
    let mut count = 0
    count += 1
    println("{} {}", name, count)
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // name should be immutable
    assert!(
        !generated.contains("let mut name"),
        "Expected immutable `let name`, got:\n{}",
        generated
    );

    // count should be mutable
    assert!(
        generated.contains("let mut count"),
        "Expected `let mut count`, got:\n{}",
        generated
    );
}

// ============================================================================
// TEST 8: Parameter mutability inference is UNCHANGED
//
// The auto-mut for parameters (ownership inference) should still work.
// Only local `let` bindings lose auto-mut.
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_parameter_mut_inference_unchanged() {
    let source = r#"
struct Counter {
    value: int
}

impl Counter {
    fn increment(self) {
        self.value += 1
    }
}

fn main() {
    let mut c = Counter { value: 0 }
    c.increment()
}
"#;

    let (generated, _stderr) = compile_wj(source);

    // The `self` parameter should still be auto-inferred as `&mut self`
    assert!(
        generated.contains("&mut self"),
        "Parameter ownership inference should still work. Got:\n{}",
        generated
    );
}
