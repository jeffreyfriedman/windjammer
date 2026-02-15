//! TDD test for automatic String → &str coercion in function calls.
//!
//! Bug: When a function expects &str and receives a String (e.g., from format!()),
//! the compiler should automatically add .as_str() or & to coerce.
//! Currently it generates `draw_text(format!(...))` instead of `draw_text(&format!(...))`.
//!
//! THE WINDJAMMER WAY: The compiler handles mechanical details like borrowing.
//! Users shouldn't need to think about String vs &str coercion.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_and_check_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!("wj_str_borrow_test_{}_{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).unwrap();
    
    let source_path = tmp_dir.join("test.wj");
    std::fs::write(&source_path, source).unwrap();
    
    let output_dir = tmp_dir.join("output");
    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", source_path.to_str().unwrap(), "--target", "rust", "--output", output_dir.to_str().unwrap(), "--no-cargo"])
        .output()
        .expect("failed to run wj");
    
    let rs_path = output_dir.join("test.rs");
    let generated = std::fs::read_to_string(&rs_path)
        .unwrap_or_else(|_| panic!("Failed to read generated Rust at {:?}", rs_path));
    
    // Now try to compile the generated Rust
    let compile_result = Command::new("rustc")
        .args(["--edition", "2021", &rs_path.to_str().unwrap(), "--crate-type", "lib", "-o", "/dev/null"])
        .output()
        .expect("failed to run rustc");
    
    let compiles = compile_result.status.success();
    
    let _ = std::fs::remove_dir_all(&tmp_dir);
    (generated, compiles)
}

#[test]
fn test_format_string_passed_to_str_param() {
    // When format!() (producing String) is passed to a function expecting &str,
    // the generated Rust should auto-borrow with &
    let source = r#"
fn greet(name: &str) {
    println!("Hello, {}!", name)
}

fn main() {
    let x = 42
    greet(format!("World #{}", x))
}
"#;
    let (generated, _compiles) = compile_and_check_rust(source);
    
    // The generated code should have & before format! to borrow the String as &str
    // Acceptable forms: &format!(...), format!(...).as_str(), or &*format!(...)
    assert!(
        generated.contains("&format!") || generated.contains(".as_str()"),
        "Expected auto-borrow of format!() when passed to &str parameter.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_string_literal_to_str_param_no_extra_borrow() {
    // String literals are already &str in Rust, no extra borrowing needed
    let source = r#"
fn greet(name: &str) {
    println!("Hello, {}!", name)
}

fn main() {
    greet("World")
}
"#;
    let (generated, _compiles) = compile_and_check_rust(source);
    
    // Should NOT double-borrow a string literal
    assert!(
        !generated.contains("&\"World\"") && !generated.contains("&&"),
        "Should not double-borrow string literals.\nGenerated:\n{}",
        generated
    );
}
