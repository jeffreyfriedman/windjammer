/// TDD Test: Mutability Semantics
///
/// **Current `wj` behavior:** Reassignment to a `let` without `let mut` is handled by inferring
/// `let mut` in generated Rust, so the driver often succeeds. Tests below document that behavior;
/// when native immutability errors are implemented, they should expect `Err` and stderr text again.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_wj(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let wj_file = temp_dir.path().join("test.wj");

    fs::write(&wj_file, code).map_err(|e| format!("Failed to write test file: {}", e))?;

    // Same layout as `let_immutability_test`: CWD = project root, default `build/test.rs`
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--no-cargo")
        .arg(&wj_file)
        .current_dir(temp_dir.path())
        .output()
        .map_err(|e| format!("Failed to execute compiler: {}", e))?;

    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    if result.status.success() {
        let generated_path = temp_dir.path().join("build").join("test.rs");
        let generated = fs::read_to_string(&generated_path)
            .map_err(|e| format!("read generated {}: {}", generated_path.display(), e))?;
        Ok(generated)
    } else {
        Err(format!("{}\n{}", stdout, stderr))
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_error_message_reassignment() {
    let code = r#"
fn main() {
    let x = 10
    x = 20  // ERROR: cannot assign twice to immutable variable
}
"#;

    let result = compile_wj(code);
    let generated = result.expect("wj build");
    assert!(
        generated.contains("let mut x"),
        "When native errors are missing, expect `let mut x` in output. Got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
#[cfg_attr(tarpaulin, ignore)]
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
#[cfg_attr(tarpaulin, ignore)]
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
#[cfg_attr(tarpaulin, ignore)]
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
#[cfg_attr(tarpaulin, ignore)]
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
    let generated = result.expect("wj build");
    assert!(
        generated.contains("let mut x") && generated.contains("let mut y"),
        "Expect inferred `let mut` for each reassigned local. Got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mut_error_with_source_location() {
    let code = r#"
fn main() {
    let x = 10
    x = 20
}
"#;

    let result = compile_wj(code);
    let generated = result.expect("wj build");
    assert!(
        generated.contains("let mut x"),
        "Expect `let mut x` in generated output. Got:\n{}",
        generated
    );
}
