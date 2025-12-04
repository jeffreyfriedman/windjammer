// Integration test: String → &str borrow inference
// Verifies that String parameters are automatically inferred as &str when only read

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_string_borrow_inference() {
    let test_file = PathBuf::from("tests/string_borrow_inference_test.wj");
    let output_dir = PathBuf::from("target/test_output/string_borrow_inference");

    // Clean output directory
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).unwrap();

    // Compile the test file
    let output = Command::new("cargo")
        .args(["run", "--release", "--", "build"])
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        eprintln!(
            "Compiler stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        eprintln!(
            "Compiler stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        panic!("Compilation failed");
    }

    // Read generated Rust code
    let generated_file = output_dir.join("string_borrow_inference_test.rs");
    let generated_code =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");

    println!("Generated code:\n{}", generated_code);

    // Verify log_file infers &str (read-only parameter)
    assert!(
        generated_code.contains("pub fn log_file(path: &str)"),
        "log_file should infer path: &str (parameter is only read)"
    );

    // Verify log_message uses &str parameter (explicitly declared in source)
    assert!(
        generated_code.contains("pub fn log_message(&self, message: &str)"),
        "log_message should have &self and message: &str"
    );

    // Verify new keeps String (parameter is stored)
    assert!(
        generated_code.contains("pub fn new(name: String) -> Logger"),
        "new should keep name: String (parameter is stored in struct)"
    );

    // Verify the generated code compiles with rustc
    let rustc_output = Command::new("rustc")
        .arg("--crate-type")
        .arg("lib")
        .arg(&generated_file)
        .arg("--out-dir")
        .arg(&output_dir)
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        eprintln!(
            "Rustc stderr: {}",
            String::from_utf8_lossy(&rustc_output.stderr)
        );
        panic!("Generated Rust code does not compile");
    }

    println!("✓ String borrow inference test passed!");
}
