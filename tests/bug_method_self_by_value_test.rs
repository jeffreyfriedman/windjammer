// TDD Test Harness: Bug #1 - Method self-by-value incorrectly infers &mut
// This is the Rust test harness that compiles and runs the Windjammer test

use std::fs;
use std::process::Command;

#[test]
fn test_method_self_by_value_compiles() {
    // Compile the Windjammer test file
    let output = Command::new("cargo")
        .args(&["run", "--release", "--bin", "wj", "--", "build", "tests/bug_method_self_by_value.wj", "--target", "/tmp/bug_method_self_by_value"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute wj build");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Windjammer compilation failed:\nstdout:\n{}\nstderr:\n{}",
            stdout, stderr
        );
    }

    // Check the generated Rust code
    let rust_code = fs::read_to_string("/tmp/bug_method_self_by_value/src/main.rs")
        .expect("Failed to read generated Rust code");

    // The bug manifests as: methods taking `self` by value generate `&mut self` at call site
    // We should NOT see patterns like: `let mut transform = ...` when transform is only passed to methods
    // The generated code should pass `self` by value, not `&mut self`

    println!("Generated Rust code:\n{}", rust_code);

    // Verify the generated Rust compiles
    let rust_compile = Command::new("cargo")
        .args(&["build", "--release"])
        .current_dir("/tmp/bug_method_self_by_value")
        .output()
        .expect("Failed to compile generated Rust");

    if !rust_compile.status.success() {
        let stderr = String::from_utf8_lossy(&rust_compile.stderr);
        panic!("Generated Rust failed to compile:\n{}", stderr);
    }

    // Run the generated binary and check output
    let run_output = Command::new("/tmp/bug_method_self_by_value/target/release/bug_method_self_by_value")
        .output()
        .expect("Failed to run generated binary");

    if !run_output.status.success() {
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        panic!(
            "Generated binary failed:\nstdout:\n{}\nstderr:\n{}",
            stdout, stderr
        );
    }

    println!("✅ Test passed: Method self-by-value works correctly!");
}
