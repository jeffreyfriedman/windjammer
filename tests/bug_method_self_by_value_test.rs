// TDD Test Harness: Bug #1 - Method self-by-value incorrectly infers &mut
// This is the Rust test harness that compiles and runs the Windjammer test

use std::fs;
use std::process::Command;

#[test]
fn test_method_self_by_value_compiles() {
    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();
    
    let wj_file = test_dir.join("test.wj");
    let wj_source = fs::read_to_string("tests/bug_method_self_by_value.wj")
        .expect("Failed to read source .wj file");
    fs::write(&wj_file, wj_source).unwrap();
    
    let out_dir = test_dir.join("out");
    
    // Compile the Windjammer test file
    let output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
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
    let rust_code = fs::read_to_string(out_dir.join("test.rs"))
        .expect("Failed to read generated Rust code");

    // The bug manifests as: methods taking `self` by value generate `&mut self` at call site
    // We should NOT see patterns like: `let mut transform = ...` when transform is only passed to methods
    // The generated code should pass `self` by value, not `&mut self`

    println!("Generated Rust code:\n{}", rust_code);

    // Verify the generated Rust compiles
    let rust_compile = Command::new("rustc")
        .arg(out_dir.join("test.rs"))
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to compile generated Rust");

    if !rust_compile.status.success() {
        let stderr = String::from_utf8_lossy(&rust_compile.stderr);
        panic!("Generated Rust failed to compile:\n{}\n\nGenerated code:\n{}", stderr, rust_code);
    }

    println!("✅ Test passed: Method self-by-value works correctly!");
    
    fs::remove_dir_all(&test_dir).ok();
}
