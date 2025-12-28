// Integration test for static method inference bug
// Verifies that constructors like new(), from_*(), zero() don't get &self

use std::process::Command;

#[test]
fn test_static_method_inference() {
    // Compile the Windjammer test file
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "tests/static_method_inference_test.wj",
            "--output",
            "/tmp/wj_static_method_test",
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run windjammer compiler");

    assert!(
        output.status.success(),
        "Windjammer compilation failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the generated Rust code
    let generated_code =
        std::fs::read_to_string("/tmp/wj_static_method_test/static_method_inference_test.rs")
            .expect("Failed to read generated code");

    // Verify static methods do NOT have &self
    assert!(
        generated_code.contains("pub fn new(x: f32, y: f32) -> Point"),
        "new() should not have &self parameter"
    );

    assert!(
        generated_code.contains("pub fn from_coords(x: f32, y: f32) -> Point"),
        "from_coords() should not have &self parameter"
    );

    assert!(
        generated_code.contains("pub fn zero() -> Point"),
        "zero() should not have &self parameter"
    );

    // Verify instance method DOES have &self
    assert!(
        generated_code.contains("pub fn distance(&self, other: Point) -> f32"),
        "distance() should have &self parameter"
    );

    // THE BUG: Grid::new() should NOT have &self even though it uses field names in struct literal
    assert!(
        generated_code.contains("pub fn new(width: i32, height: i32) -> Grid"),
        "Grid::new() should not have &self parameter (this is the bug!)"
    );

    // Verify the generated code compiles with rustc
    let rustc_output = Command::new("rustc")
        .args([
            "/tmp/wj_static_method_test/static_method_inference_test.rs",
            "--crate-type",
            "lib",
            "--out-dir",
            "/tmp/wj_static_method_test",
        ])
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        eprintln!("Generated Rust code:");
        eprintln!("{}", generated_code);
        eprintln!("\nRustc errors:");
        eprintln!("{}", String::from_utf8_lossy(&rustc_output.stderr));
        panic!("Generated Rust code failed to compile");
    }

    // Cleanup
    let _ = std::fs::remove_dir_all("/tmp/wj_static_method_test");
}
