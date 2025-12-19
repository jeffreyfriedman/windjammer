// Test that use crate:: paths are NOT transformed to use super::crate::

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_use_crate_path_preserved() {
    let wj_code = r#"
use crate::ffi

pub struct TestStruct {
    value: int,
}
"#;

    let output_dir = PathBuf::from("./build/tests/use_crate");
    let wj_file_path = output_dir.join("use_crate_test.wj");
    let rs_file_path = output_dir.join("use_crate_test.rs");

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

    // Check that it generated "use crate::ffi;" not "use super::crate::ffi;"
    assert!(
        generated_rust.contains("use crate::ffi;"),
        "Should generate 'use crate::ffi;' (not transformed).\nGenerated:\n{}",
        generated_rust
    );

    assert!(
        !generated_rust.contains("super::crate"),
        "Should NOT generate 'super::crate' (invalid Rust).\nGenerated:\n{}",
        generated_rust
    );

    fs::remove_file(&wj_file_path).ok();
    fs::remove_file(&rs_file_path).ok();
}

