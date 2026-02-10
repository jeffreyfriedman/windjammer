/// TDD Test: Explicit String parameters should remain owned, not be inferred as &String
///
/// BUG: When a Windjammer function declares `path: String`, the compiler is generating
/// `path: &String` in the Rust output, which causes type mismatches when passing the
/// parameter to FFI functions that expect `String`.
///
/// THE WINDJAMMER WAY: Explicit ownership should be respected!
/// - Windjammer: `fn load(path: String)` should generate Rust: `fn load(path: String)`
/// - NOT: `fn load(path: &String)`
use std::process::Command;
use tempfile::TempDir;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_string_parameter_stays_owned() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path();

    let wj_code = r#"
    extern fn consume_string(s: String) -> i64 {}
    
    pub struct Loader {}
    
    impl Loader {
        // CRITICAL: path is explicitly declared as String (owned)
        pub fn load(path: String) -> i64 {
            consume_string(path)  // Should pass String directly
        }
    }
    "#;

    // Compile the code
    let wj_file = output_dir.join("test.wj");
    std::fs::write(&wj_file, wj_code).expect("Failed to write test file");

    let compile_result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run compiler");

    assert!(
        compile_result.status.success(),
        "Compilation failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compile_result.stdout),
        String::from_utf8_lossy(&compile_result.stderr)
    );

    // Read generated Rust code
    let generated_rust =
        std::fs::read_to_string(output_dir.join("test.rs")).expect("Failed to read generated Rust");

    // CRITICAL ASSERTION: load should take String (owned), NOT &String
    assert!(
        generated_rust.contains("pub fn load(path: String)"),
        "Expected 'pub fn load(path: String)' but got different signature.\nGenerated:\n{}",
        generated_rust
    );

    // Verify it compiles with rustc (no type errors)
    let rustc_result = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg("--emit")
        .arg("metadata")
        .arg(output_dir.join("test.rs"))
        .arg("--out-dir")
        .arg(output_dir)
        .arg("--edition")
        .arg("2021")
        .output()
        .expect("Failed to run rustc");

    assert!(
        rustc_result.status.success(),
        "Generated code should compile!\nrustc stderr:\n{}",
        String::from_utf8_lossy(&rustc_result.stderr)
    );
}
