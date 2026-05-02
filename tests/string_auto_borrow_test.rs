//! TDD test for automatic String → &str coercion in function calls.
//!
//! Bug: When a function expects &str and receives a String (e.g., from format!()),
//! the compiler should automatically add .as_str() or & to coerce.
//! Currently it generates `draw_text(format!(...))` instead of `draw_text(&format!(...))`.
//!
//! THE WINDJAMMER WAY: The compiler handles mechanical details like borrowing.
//! Users shouldn't need to think about String vs &str coercion.

#[path = "test_utils.rs"]
mod test_utils;

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
    let (generated, _compiles) = test_utils::compile_single_check(source);

    // The generated code should have & before format! to borrow the String as &str
    // Acceptable forms: &format!(...), format!(...).as_str(), &_temp variable, or &*format!(...)
    assert!(
        generated.contains("&format!")
            || generated.contains(".as_str()")
            || (generated.contains("format!(") && generated.contains("&_temp")),
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
    let (generated, _compiles) = test_utils::compile_single_check(source);

    // Should NOT double-borrow a string literal
    assert!(
        !generated.contains("&\"World\"") && !generated.contains("&&"),
        "Should not double-borrow string literals.\nGenerated:\n{}",
        generated
    );
}
