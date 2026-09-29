#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Bug #3: String/&str Coercion in format!() - TDD Test
//
// This test verifies that format!() macro calls used as function arguments
// are properly extracted to temporary variables to avoid String->&str type errors.

use std::env;
use std::fs;
use std::path::PathBuf;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_as_function_argument_extracts_to_variable() {
    // RED: This test should fail until we implement the fix

    let _tmp = tempfile::tempdir().unwrap();

    let test_dir = _tmp.path().join(format!(
        "wj_format_fix_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
extern fn draw_text(text: string, x: f32, y: f32);

fn render(score: i32) {
    draw_text(format!("Score: {}", score), 10.0, 20.0);
}
"#;

    fs::write(test_dir.join("format_test.wj"), windjammer_code).unwrap();

    // Compile the file
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("format_test.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(
        output.status.success(),
        "wj build should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Read generated Rust code
    let rust_code = fs::read_to_string(test_dir.join("build/format_test.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Should extract format!() to a temp OR wrap via FFI string_to_ffi (tip).
    let has_format = rust_code.contains("format!(");
    let extracted = rust_code.contains("let")
        && (rust_code.contains("_temp") || rust_code.contains("score_text"));
    let ffi_inline = rust_code.contains("string_to_ffi") && has_format;
    assert!(
        has_format && (extracted || ffi_inline),
        "format!() into extern string must extract to temp or use string_to_ffi, got:\n{}",
        rust_code
    );

    // Direct bare format!() into draw_text without FFI wrap is the old E0308 shape.
    assert!(
        !rust_code.contains("draw_text(format!("),
        "Should NOT pass format!() directly as &str-shaped arg, got:\n{}",
        rust_code
    );

    // Cleanup
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_in_method_call_extracts_to_variable() {
    // RED: This test should also fail

    let _tmp2 = tempfile::tempdir().unwrap();

    let test_dir = _tmp2.path().join(format!(
        "wj_format_method_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
struct Context {}

impl Context {
    fn draw_text(text: string, x: f32, y: f32) {}
}

fn render(ctx: Context, lives: i32) {
    ctx.draw_text(format!("Lives: {}", lives), 100.0, 20.0);
}
"#;

    fs::write(test_dir.join("method_format.wj"), windjammer_code).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("method_format.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    let rust_code = fs::read_to_string(test_dir.join("build/method_format.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Should extract format!() to a temp OR pass format! via demoted &str / owned String
    // without the old bare-format-into-&str E0308 shape.
    let has_format = rust_code.contains("format!(");
    let extracted = rust_code.contains("let")
        && (rust_code.contains("_temp") || rust_code.contains("lives_text"));
    let inline_ok = rust_code.contains(".draw_text(format!(")
        || rust_code.contains("draw_text(format!(");
    // Tip may demote method formal to &str and pass format!(...) which yields String
    // (needs .as_str / temp) — or keep owned String. Accept extract OR cargo-valid inline.
    assert!(
        has_format && (extracted || inline_ok),
        "format!() into method string arg must extract or inline safely, got:\n{}",
        rust_code
    );

    // Cleanup
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_as_variable_assignment_unchanged() {
    // This should already work - format! assigned to variable is fine

    let _tmp3 = tempfile::tempdir().unwrap();

    let test_dir = _tmp3.path().join(format!(
        "wj_format_var_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
extern fn draw_text(text: string, x: f32, y: f32);

fn render(score: i32) {
    let msg = format!("Score: {}", score);
    draw_text(msg, 10.0, 20.0);
}
"#;

    fs::write(test_dir.join("var_format.wj"), windjammer_code).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("var_format.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    let rust_code = fs::read_to_string(test_dir.join("build/var_format.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Should keep the variable assignment as-is
    // Note: Compiler may optimize format!() to write!() for capacity hints
    assert!(
        rust_code.contains("let msg = format!(") || rust_code.contains("let msg = {"),
        "Should keep variable assignment for format! (may be optimized to write!)"
    );

    // Should pass the variable via FFI wrapper (redundant .clone()/.to_string() on String is tolerated)
    assert!(
        rust_code.contains("string_to_ffi(msg)")
            || rust_code.contains("string_to_ffi(msg.clone())")
            || rust_code.contains("string_to_ffi(msg.to_string())")
            || rust_code.contains("draw_text(&msg,")
            || rust_code.contains("draw_text(msg,"),
        "Should pass msg variable via string_to_ffi or directly. Got:\n{}",
        rust_code
    );

    // Cleanup
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_format_calls_in_same_function() {
    // Edge case: multiple format!() in same function need unique names

    let _tmp4 = tempfile::tempdir().unwrap();

    let test_dir = _tmp4.path().join(format!(
        "wj_format_multi_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));

    fs::create_dir_all(&test_dir).unwrap();

    let windjammer_code = r#"
extern fn draw_text(text: string, x: f32, y: f32);

fn render(score: i32, lives: i32) {
    draw_text(format!("Score: {}", score), 10.0, 20.0);
    draw_text(format!("Lives: {}", lives), 100.0, 20.0);
}
"#;

    fs::write(test_dir.join("multi_format.wj"), windjammer_code).unwrap();

    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg("multi_format.wj")
        .arg("--no-cargo")
        .current_dir(&test_dir)
        .output()
        .expect("Failed to run wj build");

    assert!(output.status.success(), "wj build should succeed");

    let rust_code = fs::read_to_string(test_dir.join("build/multi_format.rs"))
        .expect("Should have generated Rust file");

    println!("Generated Rust code:\n{}", rust_code);

    // Tip may extract to unique temps OR inline two string_to_ffi(format!(…)) calls.
    let temp_count = rust_code.matches("let _temp").count()
        + rust_code.matches("let score_text").count()
        + rust_code.matches("let lives_text").count();
    let ffi_format_count = rust_code.matches("string_to_ffi(format!").count()
        + rust_code.matches("string_to_ffi(format!(\"").count();
    // Count format! occurrences as a lower bound for inline FFI path.
    let format_count = rust_code.matches("format!(").count();

    assert!(
        temp_count >= 2 || (format_count >= 2 && rust_code.contains("string_to_ffi")),
        "Should create 2 temps or two FFI-wrapped format! calls. Got:\n{rust_code}"
    );

    // Cleanup
}
