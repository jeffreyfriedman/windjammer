/// TDD Test: If-else expressions in assignments should NOT have semicolons after branches
///
/// Bug: When you have `x = if cond { value1 } else { value2 }`, the generated code adds
/// semicolons after value1 and value2, causing a type mismatch error.
///
/// Expected: The if-else should be an expression with no semicolons in the branches.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_if_else_expression_in_assignment() {
    let wj_code = r#"
struct State {
    pub value: Option<i64>
}

impl State {
    pub fn update(cond: bool) {
        self.value = if cond {
            Some(42)
        } else {
            None
        }
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, wj_code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    if !result.status.success() {
        eprintln!(
            "wj build failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        panic!("Transpilation should succeed");
    }

    // Read generated code
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Should find generated Rust file");

    // The if-else should be an expression without semicolons after the branches
    // Should generate:
    //   self.value = if cond {
    //       Some(42)
    //   } else {
    //       None
    //   };
    //
    // NOT:
    //   self.value = if cond {
    //       Some(42);  <-- BAD semicolon
    //   } else {
    //       None;      <-- BAD semicolon
    //   };

    assert!(rust_code.contains("if cond"), "Should have if statement");
    assert!(rust_code.contains("Some(42)"), "Should have Some(42)");
    assert!(rust_code.contains("None"), "Should have None");

    // The key check: should NOT have semicolons after the option values
    assert!(
        !rust_code.contains("Some(42);"),
        "Should NOT have semicolon after Some(42)"
    );

    // Check for "None;" but allow "None," (which might appear in other contexts like derives)
    // We specifically want to catch "None;\n" which indicates it's a statement, not an expression
    let has_bad_none_semicolon = rust_code.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "None;" || trimmed.starts_with("None;")
    });
    assert!(
        !has_bad_none_semicolon,
        "Should NOT have semicolon after None in if-else expression branch"
    );

    // Verify it compiles with rustc
    let compile_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(&rust_file)
        .arg("-o")
        .arg(output_dir.join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !compile_result.status.success() {
        eprintln!("Generated Rust code:\n{}", rust_code);
        eprintln!(
            "Rust compilation failed:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Generated code should compile with rustc");
    }
}

#[test]
fn test_if_else_expression_in_let_binding() {
    let wj_code = r#"
pub fn get_value(cond: bool) -> Option<i64> {
    let result = if cond {
        Some(123)
    } else {
        None
    }
    result
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, wj_code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj build");

    if !result.status.success() {
        eprintln!(
            "wj build failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        );
        panic!("Transpilation should succeed");
    }

    // Read generated code
    let rust_file = output_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rust_file).expect("Should find generated Rust file");

    // Should NOT have semicolons after the branch values
    assert!(
        !rust_code.contains("Some(123);"),
        "Should NOT have semicolon after Some(123)"
    );

    let has_bad_none_semicolon = rust_code.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == "None;" || trimmed.starts_with("None;")
    });
    assert!(
        !has_bad_none_semicolon,
        "Should NOT have semicolon after None in if-else expression"
    );

    // Verify it compiles with rustc
    let compile_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--edition")
        .arg("2021")
        .arg(&rust_file)
        .arg("-o")
        .arg(output_dir.join("test.rlib"))
        .output()
        .expect("Failed to run rustc");

    if !compile_result.status.success() {
        eprintln!("Generated Rust code:\n{}", rust_code);
        eprintln!(
            "Rust compilation failed:\n{}",
            String::from_utf8_lossy(&compile_result.stderr)
        );
        panic!("Generated code should compile with rustc");
    }
}
