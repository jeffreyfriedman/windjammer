// Integration test: std::ops imports should map to Rust std::ops, not windjammer_runtime

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_std_ops_imports_map_to_rust_stdlib() {
    let wj_file = PathBuf::from("tests/std_ops_import_test.wj");
    let output_dir = PathBuf::from("./build/tests/std_ops_test");

    // Clean output directory
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir).ok();
    }
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Get compiler path - use the test-built binary
    let wj_compiler = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    // Compile the test file
    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj compiler");

    // Check compilation succeeded
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Compilation failed:\nSTDOUT: {}\nSTDERR: {}",
            stdout, stderr
        );
    }

    // Read the generated Rust file
    let generated_file = output_dir.join("std_ops_import_test.rs");
    let generated_code =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated Rust file");

    // CRITICAL ASSERTIONS:
    // 1. Must contain "use std::ops::Add;"
    assert!(
        generated_code.contains("use std::ops::Add;"),
        "Generated code should contain 'use std::ops::Add;'\nGenerated code:\n{}",
        generated_code
    );

    // 2. Must contain "use std::ops::Sub;"
    assert!(
        generated_code.contains("use std::ops::Sub;"),
        "Generated code should contain 'use std::ops::Sub;'"
    );

    // 3. Must contain "use std::ops::Mul;"
    assert!(
        generated_code.contains("use std::ops::Mul;"),
        "Generated code should contain 'use std::ops::Mul;'"
    );

    // 4. Must contain "use std::ops::Div;"
    assert!(
        generated_code.contains("use std::ops::Div;"),
        "Generated code should contain 'use std::ops::Div;'"
    );

    // 5. Must NOT contain "windjammer_runtime::ops"
    assert!(
        !generated_code.contains("windjammer_runtime::ops"),
        "Generated code should NOT contain 'windjammer_runtime::ops'\nGenerated code:\n{}",
        generated_code
    );

    // Verify the generated Rust code compiles with rustc
    let rustc_output = Command::new("rustc")
        .arg("--crate-type")
        .arg("bin")
        .arg(&generated_file)
        .arg("--out-dir")
        .arg(&output_dir)
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!("Generated Rust code failed to compile:\n{}", stderr);
    }

    // Clean up
    std::fs::remove_dir_all(&output_dir).ok();
}
