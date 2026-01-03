//! TDD Test: Extern fn declarations for FFI
//! WINDJAMMER PHILOSOPHY: Enable seamless Rust interop through extern functions
//! Extern functions allow calling Rust code (or C code via Rust) from Windjammer

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_file = test_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_file).expect("Failed to read generated file");

    Ok(generated)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_simple() {
    // TDD: Simple extern function declaration
    let code = r#"
    extern fn printf(format: &str);
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    // Should generate Rust extern "C" declaration
    assert!(
        generated.contains("extern \"C\""),
        "Should generate extern \"C\" block. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn printf(format: &str);"),
        "Should declare printf function. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_with_return_type() {
    // TDD: Extern function with return type
    let code = r#"
    extern fn malloc(size: int) -> int;
    extern fn strlen(s: &str) -> int;
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn malloc(size: i64) -> i64;"),
        "Should generate malloc with return type. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("fn strlen(s: &str) -> i64;"),
        "Should generate strlen with return type. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_multiple_params() {
    // TDD: Extern function with multiple parameters
    let code = r#"
    extern fn memcpy(dest: int, src: int, n: int) -> int;
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn memcpy(dest: i64, src: i64, n: i64) -> i64;"),
        "Should generate memcpy with multiple params. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_used_in_code() {
    // TDD: Extern function can be called
    let code = r#"
    extern fn printf(format: &str);
    
    pub fn hello() {
        printf("Hello, World!")
    }
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("extern \"C\""),
        "Should have extern C block. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("printf(\"Hello, World!\")"),
        "Should call printf. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_multiple_declarations() {
    // TDD: Multiple extern functions
    let code = r#"
    extern fn printf(format: &str);
    extern fn malloc(size: int) -> int;
    extern fn free(ptr: int);
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn printf"),
        "Should have printf. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn malloc"),
        "Should have malloc. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn free"),
        "Should have free. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_with_generics() {
    // TDD: Extern function with generic parameters
    let code = r#"
    extern fn process_data<T>(data: &T);
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn process_data"),
        "Should generate process_data. Generated:\n{}",
        generated
    );

    assert!(
        generated.contains("<T>"),
        "Should preserve generic parameter. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_extern_fn_semicolon_optional() {
    // TDD: Semicolons should be optional (Windjammer philosophy)
    let code = r#"
    extern fn func1(x: int)
    extern fn func2(y: int);
    "#;

    let generated = compile_code(code).expect("Compilation failed");

    assert!(
        generated.contains("fn func1"),
        "func1 without semicolon should work. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn func2"),
        "func2 with semicolon should work. Generated:\n{}",
        generated
    );
}
