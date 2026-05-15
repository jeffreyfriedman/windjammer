/// TDD Test: `if let` should use the bound variable, not the original expression
///
/// Bug: `if let Some(x) = expr` generates a match that binds `x`, but then the body
/// still uses `expr` instead of the bound variable `x`.
///
/// Expected: The bound variable should be used in the match arm body.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_uses_bound_variable() {
    let code = r#"
pub struct State {
    pub value: Option<i32>
}

impl State {
    pub fn process() -> i32 {
        if let Some(v) = self.value {
            v + 1
        } else {
            0
        }
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).unwrap();

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

    println!("Generated code:\n{}", rust_code);

    // The generated match arm should use the bound variable 'v', not 'self.value'
    // Should generate: Some(v) => { v + 1 }
    // NOT: Some(v) => { self.value + 1 }  <-- BUG

    // Verify it compiles with rustc (which will fail if using self.value on Option)
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
