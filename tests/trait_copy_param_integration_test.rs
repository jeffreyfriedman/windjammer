// Test that Copy type parameters in trait methods don't get unnecessary & references

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_trait_copy_params_not_borrowed() {
    let wj_code = r#"
pub trait Calculator {
    fn add(&self, a: f32, b: f32) -> f32
}

pub struct SimpleCalc {}

impl Calculator for SimpleCalc {
    fn add(&self, a: f32, b: f32) -> f32 {
        a + b
    }
}
"#;

    let output_dir = PathBuf::from("./build/tests/trait_copy");
    let wj_file_path = output_dir.join("trait_copy_test.wj");
    let rs_file_path = output_dir.join("trait_copy_test.rs");

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    fs::write(&wj_file_path, wj_code).expect("Failed to write .wj test file");

    let wj_compiler = std::env::var("WJ_COMPILER").unwrap_or_else(|_| {
        "/Users/jeffreyfriedman/src/wj/windjammer/target/release/wj".to_string()
    });

    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(&wj_file_path)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .arg("--target")
        .arg("rust")
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "Compilation failed: {}\nSTDOUT: {}\nSTDERR: {}",
            output.status, stdout, stderr
        );
    }

    let generated_rust = fs::read_to_string(&rs_file_path)
        .unwrap_or_else(|_| panic!("Failed to read generated Rust file: {:?}", rs_file_path));

    // Check that f32 parameters are NOT borrowed
    assert!(
        !generated_rust.contains("a: &f32") && !generated_rust.contains("b: &f32"),
        "Copy types (f32) should not be borrowed in trait methods.\nGenerated:\n{}",
        generated_rust
    );

    // Verify it compiles with rustc
    let rust_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rs_file_path)
        .arg("--out-dir")
        .arg(&output_dir)
        .output()
        .expect("Failed to run rustc");

    if !rust_output.status.success() {
        let stderr = String::from_utf8_lossy(&rust_output.stderr);
        panic!("Rustc compilation failed:\n{}", stderr);
    }

    fs::remove_file(&wj_file_path).ok();
    fs::remove_file(&rs_file_path).ok();
}
