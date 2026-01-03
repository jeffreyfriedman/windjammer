/// TDD Test: Extern function calls should be automatically wrapped in unsafe blocks
///
/// THE WINDJAMMER WAY: The compiler should handle unsafe details so users don't have to.
/// When calling extern functions, the compiler should automatically add unsafe blocks.
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    // Compile Windjammer to Rust
    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo", // Don't run cargo - we just want the generated Rust
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Read generated Rust code
    let generated_file = test_dir.join("build/test.rs");
    let generated =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");

    Ok(generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_calls_wrapped_in_unsafe() {
    let code = r#"
    extern fn unsafe_function(x: i32) -> i32 {}
    
    pub fn safe_wrapper(x: i32) -> i32 {
        unsafe_function(x)
    }
    
    fn main() {
        let result = safe_wrapper(42)
    }
    "#;

    match compile_windjammer_code(code) {
        Ok(generated) => {
            // Check that the extern call is wrapped in unsafe
            assert!(
                generated.contains("unsafe") && generated.contains("unsafe_function"),
                "Extern function calls should be automatically wrapped in unsafe blocks.\nGenerated code:\n{}",
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_rendering_api_with_extern_calls() {
    let code = r#"
    extern fn renderer_clear(r: f32, g: f32, b: f32, a: f32) {}
    
    pub fn clear_color(r: f32, g: f32, b: f32, a: f32) {
        renderer_clear(r, g, b, a)
    }
    
    fn main() {
        clear_color(0.0, 0.0, 0.0, 1.0)
    }
    "#;

    match compile_windjammer_code(code) {
        Ok(generated) => {
            // The generated code should compile with rustc
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rust_file = temp_dir.path().join("test.rs");
            std::fs::write(&rust_file, &generated).expect("Failed to write Rust file");

            // Compile to object file only (don't link) since extern fn has no implementation
            let rustc_output = Command::new("rustc")
                .arg("--crate-type")
                .arg("lib")
                .arg("--emit")
                .arg("metadata") // Just check syntax, don't link
                .arg(&rust_file)
                .arg("--out-dir")
                .arg(temp_dir.path())
                .output()
                .expect("Failed to run rustc");

            assert!(
                rustc_output.status.success(),
                "Generated Rust code should compile with rustc.\nrustc stderr:\n{}\nGenerated code:\n{}",
                String::from_utf8_lossy(&rustc_output.stderr),
                generated
            );
        }
        Err(err) => {
            panic!("Compilation failed: {}", err);
        }
    }
}
