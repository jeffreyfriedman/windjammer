/// TDD Test: `if let` bound variables should be used in method calls
///
/// Bug: When `if let Some(x) = self.field` is used and then x.method() is called,
/// the generated code incorrectly uses self.field.method() instead of x.method().
///
/// This causes "no method found for enum `Option<T>`" errors.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_let_bound_variable_method_calls() {
    let code = r#"
pub struct SearchState {
    pub timer: f32
}

impl SearchState {
    pub fn update(dt: f32) -> bool {
        self.timer -= dt
        self.timer > 0.0
    }
    
    pub fn is_done() -> bool {
        self.timer <= 0.0
    }
}

pub struct AIState {
    pub search: Option<SearchState>
}

impl AIState {
    pub fn tick(dt: f32) {
        if let Some(state) = &mut self.search {
            if state.update(dt) || state.is_done() {
                self.search = None
            }
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

    // The key bug: should use 'state.update(dt)' not 'self.search.update(dt)'
    // When the match arm binds Some(state), the body should use 'state'
    assert!(!rust_code.contains("self.search.update("), 
        "Should NOT call methods on self.search (it's an Option). Should use bound variable 'state'.");
    assert!(!rust_code.contains("self.search.is_done("), 
        "Should NOT call methods on self.search (it's an Option). Should use bound variable 'state'.");

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
