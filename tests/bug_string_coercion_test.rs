// Bug #3: String/&str Coercion in format!() - TDD Test
//
// This test verifies that format!() macro calls used as function arguments
// are properly extracted to temporary variables to avoid String->&str type errors.

use std::fs;
use std::path::PathBuf;
use std::env;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_as_function_argument_extracts_to_variable() {
    // RED: This test should fail until we implement the fix
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_format_fix_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();
    
    let windjammer_code = r#"
extern fn draw_text(text: &str, x: f32, y: f32);

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
    
    assert!(output.status.success(), 
            "wj build should succeed, stderr: {}",
            String::from_utf8_lossy(&output.stderr));
    
    // Read generated Rust code
    let rust_code = fs::read_to_string(test_dir.join("build/format_test.rs"))
        .expect("Should have generated Rust file");
    
    println!("Generated Rust code:\n{}", rust_code);
    
    // Should extract format!() to a variable
    assert!(
        rust_code.contains("let") && rust_code.contains("format!("),
        "Should extract format!() to a variable, got:\n{}",
        rust_code
    );
    
    // Should pass reference to the variable
    assert!(
        rust_code.contains("&_temp") || rust_code.contains("&score_text"),
        "Should pass reference to temp variable, got:\n{}",
        rust_code
    );
    
    // Should NOT pass format!() directly as argument
    assert!(
        !rust_code.contains("draw_text(format!("),
        "Should NOT pass format!() directly as argument, got:\n{}",
        rust_code
    );
    
    // Cleanup
    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_in_method_call_extracts_to_variable() {
    // RED: This test should also fail
    
    let test_dir = std::env::temp_dir().join(format!(
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
    fn draw_text(text: &str, x: f32, y: f32) {}
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
    
    assert!(output.status.success(), 
            "wj build should succeed");
    
    let rust_code = fs::read_to_string(test_dir.join("build/method_format.rs"))
        .expect("Should have generated Rust file");
    
    println!("Generated Rust code:\n{}", rust_code);
    
    // Should extract format!() to a variable
    assert!(
        rust_code.contains("let") && rust_code.contains("format!("),
        "Should extract format!() to a variable for method calls too"
    );
    
    // Should pass reference
    assert!(
        rust_code.contains("&_temp") || rust_code.contains("&lives_text"),
        "Should pass reference to temp variable"
    );
    
    // Should NOT pass format!() directly
    assert!(
        !rust_code.contains("draw_text(format!(") && !rust_code.contains(".draw_text(format!("),
        "Should NOT pass format!() directly as argument"
    );
    
    // Cleanup
    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_format_as_variable_assignment_unchanged() {
    // This should already work - format! assigned to variable is fine
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_format_var_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();
    
    let windjammer_code = r#"
extern fn draw_text(text: &str, x: f32, y: f32);

fn render(score: i32) {
    let msg = format!("Score: {}", score);
    draw_text(&msg, 10.0, 20.0);
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
    
    assert!(output.status.success(), 
            "wj build should succeed");
    
    let rust_code = fs::read_to_string(test_dir.join("build/var_format.rs"))
        .expect("Should have generated Rust file");
    
    println!("Generated Rust code:\n{}", rust_code);
    
    // Should keep the variable assignment as-is
    // Note: Compiler may optimize format!() to write!() for capacity hints
    assert!(
        rust_code.contains("let msg = format!(") || rust_code.contains("let msg = {"),
        "Should keep variable assignment for format! (may be optimized to write!)"
    );
    
    // Should pass the variable (with or without &)
    assert!(
        rust_code.contains("draw_text(&msg,") || rust_code.contains("draw_text(msg,"),
        "Should pass msg variable"
    );
    
    // Cleanup
    fs::remove_dir_all(test_dir).ok();
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_format_calls_in_same_function() {
    // Edge case: multiple format!() in same function need unique names
    
    let test_dir = std::env::temp_dir().join(format!(
        "wj_format_multi_{}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::process::id()
    ));
    fs::create_dir_all(&test_dir).unwrap();
    
    let windjammer_code = r#"
extern fn draw_text(text: &str, x: f32, y: f32);

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
    
    assert!(output.status.success(), 
            "wj build should succeed");
    
    let rust_code = fs::read_to_string(test_dir.join("build/multi_format.rs"))
        .expect("Should have generated Rust file");
    
    println!("Generated Rust code:\n{}", rust_code);
    
    // Should have 2 different temp variables
    let temp_count = rust_code.matches("let _temp").count() 
                   + rust_code.matches("let score_text").count() 
                   + rust_code.matches("let lives_text").count();
    
    assert!(
        temp_count >= 2,
        "Should create 2 separate temp variables for 2 format! calls"
    );
    
    // Cleanup
    fs::remove_dir_all(test_dir).ok();
}
