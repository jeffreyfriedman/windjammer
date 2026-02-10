// Integration test: String → &str borrow inference
// Verifies that String parameters are automatically inferred as &str when only read

use std::path::PathBuf;
use std::process::Command;

fn get_wj_compiler() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_borrow_inference() {
    let test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("string_borrow_inference_test.wj");
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_output")
        .join("string_borrow_inference");

    // Clean output directory
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).unwrap();

    // Compile the test file
    let output = Command::new(get_wj_compiler())
        .args(["build"])
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

    // Verify Logger::new has String parameter (parameter is stored in struct)
    assert!(
        generated_code.contains("fn new")
            && generated_code.contains("String")
            && generated_code.contains("Logger"),
        "new should take owned String (parameter is stored). Generated:\n{}",
        generated_code
    );

    // Verify store_path has String parameter (parameter is pushed to Vec)
    assert!(
        generated_code.contains("fn store_path") && generated_code.contains("String"),
        "store_path should take owned String (parameter is pushed). Generated:\n{}",
        generated_code
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
