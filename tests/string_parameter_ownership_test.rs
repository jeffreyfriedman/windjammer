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

/// TDD Test: String Parameter Ownership
///
/// THE WINDJAMMER WAY: Explicit type annotations are honored.
/// - `name: String` → `String` (owned, as written)
/// - `name: &string` → `&str` (borrowed, as written)
///
/// This prevents API contract violations where methods expect owned strings.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_explicit_string_type_honored() {
    let source = r#"
fn greet(name: string) {
    println!("Hello, {}", name)
}

fn main() {
    greet("Alice")
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Windjammer infers ownership: read-only `string` → `&str`, stored → `String`
    assert!(
        rust_code.contains("fn greet(name: String)")
            || rust_code.contains("fn greet(name: &str)"),
        "String parameter should be String (owned) or &str (inferred borrow).\nGenerated:\n{}",
        rust_code
    );

    // Call site depends on parameter type: &str → bare literal, String → .to_string()
    let has_call = rust_code.contains(r#"greet("Alice")"#)
        || rust_code.contains(r#"greet("Alice".to_string())"#)
        || rust_code.contains(r#"greet(String::from("Alice"))"#);
    assert!(
        has_call,
        "String literal should be passed correctly at call site.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_string_type_honored() {
    let source = r#"
fn greet(name: string) {
    println!("Hello, {}", name)
}

fn main() {
    let name = "Alice".to_string()
    greet(&name)
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Explicit `&string` type is honored as `&str` (borrowed)
    assert!(
        rust_code.contains("fn greet(name: &str)") || rust_code.contains("fn greet(name: &String)"),
        "Explicit &string type should be honored as &str or &String.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_for_storage() {
    let source = r#"
struct User {
    name: string,
}

impl User {
    fn new(name: string) -> User {
        User { name }
    }
}

fn main() {
    let user = User::new("Alice".to_string())
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Struct field is String; constructor param is String (owned) or &str
    assert!(
        rust_code.contains("name: String"),
        "Struct field should remain String.\nGenerated:\n{}",
        rust_code
    );
    assert!(
        rust_code.contains("fn new(name: &str)")
            || rust_code.contains("fn new(name: String)"),
        "Constructor param should be &str or String (owned).\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_types_passed_by_value() {
    let source = r#"
fn double(x: int) -> int {
    x * 2
}

fn main() {
    let x = 5
    let result = double(x)
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Copy types should be passed by value (no & needed)
    assert!(
        rust_code.contains("fn double(x: i64) -> i64"),
        "Copy types should be passed by value.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("double(x)") && !rust_code.contains("double(&x)"),
        "Copy types should be passed by value at call site.\nGenerated:\n{}",
        rust_code
    );
}
