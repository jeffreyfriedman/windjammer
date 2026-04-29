// TDD Test: Windjammer Compiler Bug - extern fn not being transpiled
//
// Bug: extern fn declarations in .wj files are not appearing in generated .rs files
// Expected: extern fn should be transpiled to Rust's extern "C" { pub fn ... }

use std::fs;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, source).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_path)
        .arg("--output")
        .arg(&out_dir)
        .arg("--target")
        .arg("rust")
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Compilation failed:\n{}", stderr);
    }

    let src_main = out_dir.join("src").join("main.rs");
    let test_rs = out_dir.join("test.rs");
    if src_main.exists() {
        fs::read_to_string(src_main).expect("Failed to read generated .rs file")
    } else if test_rs.exists() {
        fs::read_to_string(test_rs).expect("Failed to read generated .rs file")
    } else {
        panic!("No generated Rust file found in {:?}", out_dir);
    }
}

#[test]
fn test_extern_fn_transpiles_to_rust() {
    let test_wj = r#"
extern fn test_simple_function(x: i32) -> i32
extern fn test_no_return(value: f32)
extern fn test_multiple_params(a: u32, b: u32, c: f32) -> bool
extern fn test_string_param(path: string) -> u32
"#;

    let rust_code = compile_and_get_rust(test_wj);

    assert!(
        rust_code.contains("extern \"C\""),
        "Generated Rust should have extern \"C\" block"
    );

    assert!(
        rust_code.contains("pub fn test_simple_function(x: i32) -> i32"),
        "Should transpile: extern fn test_simple_function(x: i32) -> i32"
    );

    assert!(
        rust_code.contains("pub fn test_no_return(value: f32)"),
        "Should transpile: extern fn test_no_return(value: f32)"
    );

    assert!(
        rust_code.contains("pub fn test_multiple_params(a: u32, b: u32, c: f32) -> bool"),
        "Should transpile: extern fn test_multiple_params(a: u32, b: u32, c: f32) -> bool"
    );

    assert!(
        rust_code.contains("test_string_param") && rust_code.contains("FfiString"),
        "String parameters should become FfiString in generated Rust"
    );
}

#[test]
fn test_extern_fn_in_extern_block() {
    let test_wj = r#"
extern fn func_a(x: i32) -> i32
extern fn func_b(y: f32) -> f32
extern fn func_c()
"#;

    let rust_code = compile_and_get_rust(test_wj);

    let extern_count = rust_code.matches("extern \"C\"").count();
    assert_eq!(
        extern_count, 1,
        "Should have exactly one extern \"C\" block"
    );

    assert!(
        rust_code.contains("pub fn func_a"),
        "func_a should be in extern block"
    );
    assert!(
        rust_code.contains("pub fn func_b"),
        "func_b should be in extern block"
    );
    assert!(
        rust_code.contains("pub fn func_c"),
        "func_c should be in extern block"
    );
}
