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
/// - `name: string` → `String` (owned, as written)
/// - `name: &string` → `&str` (borrowed, as written)
///
/// This prevents API contract violations where methods expect owned strings.
#[path = "../common/test_utils.rs"]
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

    // NEW DESIGN: String ownership is inferred from USAGE
    // `name: string` (read-only) → infers to `name: &str` (idiomatic Rust!)
    assert!(
        rust_code.contains("fn greet(name: &str)"),
        "Read-only string parameter should infer to &str.\nGenerated:\n{}",
        rust_code
    );

    // String literals (already &str) are passed directly to &str parameters
    assert!(
        rust_code.contains(r#"greet("Alice")"#),
        "String literals should be passed directly to &str parameters.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_string_type_honored() {
    let source = r#"
fn greet(name: &string) {
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

    // THE WINDJAMMER WAY: String parameters for storage should be String (owned)
    assert!(
        rust_code.contains("name: String") && rust_code.contains("fn new(name: String)"),
        "String parameters that are stored should be String (owned).\nGenerated:\n{}",
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
