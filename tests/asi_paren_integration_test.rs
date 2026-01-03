// ASI Bug: Parenthesized expression on new line should have semicolon inserted before it

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_asi_before_parenthesized_expression() {
    let wj_code = r#"
pub fn test_asi() -> f32 {
    let dx = 3.0
    let dy = 4.0
    let dz = 5.0
    (dx * dx + dy * dy + dz * dz).sqrt()
}
"#;

    let output_dir = PathBuf::from("./build/tests/asi_paren");
    let wj_file_path = output_dir.join("asi_paren_test.wj");
    let rs_file_path = output_dir.join("asi_paren_test.rs");

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    fs::write(&wj_file_path, wj_code).expect("Failed to write .wj test file");

    // Use the wj binary from target/release, built by cargo
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
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

    // Check that it generated separate statements, not a function call
    assert!(
        !generated_rust.contains("dz(dx"),
        "ASI Bug: Parenthesis should not be treated as function call.\nGenerated:\n{}",
        generated_rust
    );

    // Check that dz is assigned correctly
    assert!(
        generated_rust.contains("let dz = 5.0;"),
        "ASI should insert semicolon after let statement.\nGenerated:\n{}",
        generated_rust
    );

    fs::remove_file(&wj_file_path).ok();
    fs::remove_file(&rs_file_path).ok();
}
