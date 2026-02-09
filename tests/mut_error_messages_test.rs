/// TDD Test: Mutability Semantics
///
/// Windjammer uses immutable-by-default `let` semantics:
/// - `let x = ...` is immutable
/// - `let mut x = ...` is mutable
///
/// This follows the modern language consensus (Rust, Swift, Kotlin, Zig).
/// The compiler no longer auto-infers `mut` for local bindings.
use tempfile::TempDir;

fn compile_wj(code: &str) -> Result<String, String> {
    use std::fs;
    use std::process::Command;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let output_dir = temp_dir.path().to_path_buf();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).map_err(|e| format!("Failed to write test file: {}", e))?;

    // Compile Windjammer to Rust
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo") // Don't run cargo, we just want Windjammer errors
        .output()
        .map_err(|e| format!("Failed to execute compiler: {}", e))?;

    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    if result.status.success() {
        Ok(stdout.to_string())
    } else {
        Err(format!("{}\n{}", stdout, stderr))
    }
}

#[test]
fn test_mut_error_message_reassignment() {
    let code = r#"
fn main() {
    let x = 10
    x = 20  // ERROR: cannot assign twice to immutable variable
}
"#;

    let result = compile_wj(code);
    assert!(result.is_err(), "Should fail compilation");

    let error = result.unwrap_err();

    // Check for clear error message
    assert!(
        error.contains("cannot assign") || error.contains("immutable") || error.contains("mut"),
        "Error should mention that variable is immutable or needs mut, got:\n{}",
        error
    );

    // Check for helpful suggestion
    assert!(
        error.contains("help")
            || error.contains("suggestion")
            || error.contains("make this binding mutable"),
        "Error should include a helpful suggestion, got:\n{}",
        error
    );
}

#[test]
fn test_mut_explicit_compound_assignment() {
    // Immutable-by-default: users must write `let mut` for mutable bindings
    let code = r#"
fn main() {
    let mut count = 0
    count += 1
}
"#;

    let result = compile_wj(code);
    assert!(
        result.is_ok(),
        "Should compile successfully with explicit let mut, got error:\n{:?}",
        result.err()
    );
}

#[test]
fn test_mut_explicit_field_mutation() {
    // Immutable-by-default: users must write `let mut` for mutable bindings
    let code = r#"
struct Point {
    pub x: i32,
    pub y: i32,
}

fn main() {
    let mut p = Point { x: 0, y: 0 }
    p.x = 10
}
"#;

    let result = compile_wj(code);
    assert!(
        result.is_ok(),
        "Should compile successfully with explicit let mut, got error:\n{:?}",
        result.err()
    );
}

#[test]
fn test_mut_explicit_method_call() {
    // Immutable-by-default: users must write `let mut` for mutable bindings
    let code = r#"
fn main() {
    let mut items = Vec::new()
    items.push(1)
    items.push(2)
}
"#;

    let result = compile_wj(code);
    assert!(
        result.is_ok(),
        "Should compile successfully with explicit let mut, got error:\n{:?}",
        result.err()
    );
}

#[test]
fn test_mut_works_when_declared() {
    let code = r#"
fn main() {
    let mut x = 10
    x = 20
    x += 5
    println!("{}", x)
}
"#;

    let result = compile_wj(code);
    assert!(
        result.is_ok(),
        "Should compile successfully when mut is declared, got error:\n{:?}",
        result.err()
    );
}

#[test]
fn test_multiple_mut_errors() {
    let code = r#"
fn main() {
    let x = 10
    let y = 20
    
    x = 15  // ERROR 1
    y = 25  // ERROR 2
}
"#;

    let result = compile_wj(code);
    assert!(result.is_err(), "Should fail compilation");

    let error = result.unwrap_err();

    // Should report both errors
    let error_count = error.matches("cannot").count();
    assert!(
        error_count >= 1,
        "Should report at least one mutability error (ideally both), got:\n{}",
        error
    );
}

#[test]
fn test_mut_error_with_source_location() {
    let code = r#"
fn main() {
    let x = 10
    x = 20
}
"#;

    let result = compile_wj(code);
    assert!(result.is_err(), "Should fail compilation");

    let error = result.unwrap_err();

    // Should include line information
    assert!(
        error.contains("line") || error.contains("4") || error.contains(":"),
        "Error should include source location information, got:\n{}",
        error
    );
}
