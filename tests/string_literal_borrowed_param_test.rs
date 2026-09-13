#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! String literals into plain WJ `string` formals must rustc-check.
//!
//! Product policy (IR emission contract): plain `string` formals stay owned
//! `String` until demotion is codegen-confirmed. Literals therefore need
//! `.to_string()` (ToOwnedString). If tip demotes to `&str`, bare `"World"` is OK.

use std::fs;
use std::process::Command;

#[test]
fn test_string_literal_to_borrowed_string_param() {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let wj_code = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

pub fn test_greet() -> string {
    greet("World")
}
"#;

    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_bin)
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== WJ COMPILE OUTPUT ===");
    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    let rs_file = out_dir.join("test.rs");
    let rust_code = fs::read_to_string(&rs_file).expect("Failed to read generated Rust file");

    println!("=== GENERATED RUST ===");
    println!("{}", rust_code);

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    let rustc_stderr = String::from_utf8_lossy(&rustc_output.stderr);
    println!("=== RUSTC OUTPUT ===");
    println!("{}", rustc_stderr);

    assert!(
        rust_code.contains("fn greet(name: String)") || rust_code.contains("fn greet(name: &str)"),
        "FAIL: greet formal must be String (owned default) or &str (demoted).\n\
         Generated:\n{}",
        rust_code
    );

    if rust_code.contains("fn greet(name: String)") {
        assert!(
            rust_code.contains(r#"greet("World".to_string())"#)
                || rust_code.contains(r#"greet(String::from("World"))"#),
            "FAIL: owned String formal needs owned literal coercion.\n\
             Generated:\n{}",
            rust_code
        );
    } else {
        assert!(
            rust_code.contains(r#"greet("World")"#),
            "FAIL: demoted &str formal should take bare literal.\n\
             Generated:\n{}",
            rust_code
        );
    }

    assert!(
        !rustc_stderr.contains("E0308"),
        "FAIL: Generated Rust must rustc without E0308.\n{}",
        rustc_stderr
    );
}
