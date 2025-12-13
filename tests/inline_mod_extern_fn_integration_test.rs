// Integration test: inline modules with extern fn declarations should generate correct Rust code

use std::fs;
use std::process::Command;

#[test]
#[ignore] // TODO: Implement inline module code generation (mod ffi { extern fn ... })
fn test_inline_mod_with_extern_fn_generates_correct_code() {
    let wj_compiler =
        std::env::var("WJ_COMPILER").unwrap_or_else(|_| "./target/release/wj".to_string());

    let wj_file = "tests/inline_mod_extern_fn_test.wj";

    // Compile the test file
    let output = Command::new(&wj_compiler)
        .arg("build")
        .arg(wj_file)
        .arg("--output")
        .arg("./build")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    assert!(
        output.status.success(),
        "Compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read the generated Rust file (in build/ directory, not build/tests/)
    let rs_filename = std::path::Path::new(wj_file)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .replace(".wj", ".rs");
    let generated_rust =
        fs::read_to_string(format!("./build/{}", rs_filename)).unwrap_or_else(|_| {
            panic!(
                "Failed to read generated Rust file: ./build/{}",
                rs_filename
            )
        });

    // Check that extern "C" block was generated correctly
    assert!(
        generated_rust.contains("extern \"C\""),
        "Expected extern \"C\" block in generated code:\n{}",
        generated_rust
    );

    assert!(
        generated_rust.contains("pub fn my_extern_function"),
        "Expected extern function declaration in generated code:\n{}",
        generated_rust
    );

    // Should NOT have empty TODO comment
    assert!(
        !generated_rust.contains("// TODO: Inline module support"),
        "Should not have TODO comment for inline module - should generate extern block instead:\n{}",
        generated_rust
    );

    println!("✓ inline_mod_extern_fn test passed");
}
