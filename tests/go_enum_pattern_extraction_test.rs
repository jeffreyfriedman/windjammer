//! TDD Test: Go Enum Pattern Variable Extraction
//!
//! Bug: match Maybe::Some(v) => v generates invalid Go (undefined: v)
//! Root Cause: Pattern variables not extracted from enum variant structs
//! Expected: case MaybeSome: v := _v.Value; return v

use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_go_enum_variant_value_extraction() {
    let source = r#"
enum Maybe {
    Some(int),
    None,
}

fn unwrap_or(m: Maybe, default: int) -> int {
    match m {
        Maybe::Some(v) => v,
        Maybe::None => default,
    }
}

fn main() {
    let x = unwrap_or(Maybe::Some(42), 0)
    println("${x}")
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, source).unwrap();

    // Compile to Go
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("go")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read generated Go code
    let go_file = temp_dir.path().join("build/main.go");
    let go_code = std::fs::read_to_string(&go_file)
        .expect("Failed to read generated Go file");

    // Check for proper value extraction
    // Should have: v := _v.Value (or similar extraction)
    // Should NOT have: undefined variable v
    assert!(
        go_code.contains("_v") || go_code.contains("Value"),
        "Generated Go should extract enum variant value"
    );

    // Verify Go compiles
    let go_output = Command::new("go")
        .arg("run")
        .arg("main.go")
        .current_dir(temp_dir.path().join("build"))
        .output()
        .expect("Failed to run go");

    if !go_output.status.success() {
        let stderr = String::from_utf8_lossy(&go_output.stderr);
        panic!("Go compilation failed:\n{}", stderr);
    }

    // Check output is correct
    let stdout = String::from_utf8_lossy(&go_output.stdout);
    assert!(
        stdout.contains("42"),
        "Expected output '42', got: {}",
        stdout
    );
}

#[test]
fn test_go_enum_variant_construction() {
    let source = r#"
enum Maybe {
    Some(int),
    None,
}

fn main() {
    let x = Maybe::Some(42)
    println("${x}")
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, source).unwrap();

    // Compile to Go
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg("--target")
        .arg("go")
        .arg("--no-cargo")
        .arg(&test_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute wj");

    assert!(
        output.status.success(),
        "wj build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read generated Go code
    let go_file = temp_dir.path().join("build/main.go");
    let go_code = std::fs::read_to_string(&go_file)
        .expect("Failed to read generated Go file");

    // Check for proper enum construction
    // Should NOT have: MaybeSome{}(42) - this is invalid Go syntax
    // Should have: MaybeSome{Value: 42} or NewMaybeSome(42)
    assert!(
        !go_code.contains("MaybeSome{}(42)"),
        "Generated Go should not use invalid MaybeSome{{}}(42) syntax"
    );

    // Verify Go compiles
    let go_output = Command::new("go")
        .arg("run")
        .arg("main.go")
        .current_dir(temp_dir.path().join("build"))
        .output()
        .expect("Failed to run go");

    if !go_output.status.success() {
        let stderr = String::from_utf8_lossy(&go_output.stderr);
        panic!("Go compilation failed:\n{}", stderr);
    }

    let stdout = String::from_utf8_lossy(&go_output.stdout);
    // Go prints struct as {FieldValue}, e.g., {42}
    assert!(
        stdout.contains("{42}") || stdout.contains("42"),
        "Expected output containing '42', got: {}",
        stdout
    );
}
