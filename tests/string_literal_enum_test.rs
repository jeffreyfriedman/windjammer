//! Test: String literal to String conversion in enum construction
//! Bug: Codegen produces `Speaker::NPC("string")` which generates `&str`
//! Fix: Should generate `Speaker::NPC("string".to_string())` or `Speaker::NPC(String::from("string"))`

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_string_literal_enum_auto_convert() {
    // Use cross-platform temp directory
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_enum_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    );
    let output_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&output_dir).unwrap();

    // Get paths
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let test_file = PathBuf::from(manifest_dir).join("tests/string_literal_enum_test.wj");

    // Use CARGO_BIN_EXE_wj for cross-platform compatibility
    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .args([
            "build",
            test_file.to_str().unwrap(),
            "--output",
            output_dir.to_str().unwrap(),
            "--target",
            "rust",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj build");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let _ = fs::remove_dir_all(&output_dir);
        panic!("wj build failed:\nSTDERR:\n{}\nSTDOUT:\n{}", stderr, stdout);
    }

    // Read generated Rust file
    let generated_rs = output_dir.join("string_literal_enum_test.rs");
    let generated_code =
        fs::read_to_string(&generated_rs).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated_code);

    // Verify string literal is converted to String
    // Should contain either:
    // - Speaker::NPC("Alice".to_string())
    // - Speaker::NPC(String::from("Alice"))
    assert!(
        generated_code.contains("\"Alice\".to_string()")
            || generated_code.contains("String::from(\"Alice\")"),
        "Generated code should convert string literal to String"
    );

    // Clean up temp directory
    let _ = fs::remove_dir_all(&output_dir);
}
