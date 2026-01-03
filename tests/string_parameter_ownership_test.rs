/// TDD Test: String Parameter Ownership
///
/// THE WINDJAMMER WAY: Explicit type annotations are honored.
/// - `name: string` → `String` (owned, as written)
/// - `name: &string` → `&str` (borrowed, as written)
///
/// This prevents API contract violations where methods expect owned strings.
use std::path::PathBuf;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, code).unwrap();

    let output_dir = temp_dir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let compiler_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wj_binary = compiler_path.join("target/release/wj");

    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = output_dir.join("test.rs");
    let generated_code =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated code");

    Ok(generated_code)
}

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

    let rust_code = compile_code(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Explicit `string` type is honored as `String` (owned)
    assert!(
        rust_code.contains("fn greet(name: String)"),
        "Explicit string type should be honored as String (owned).\nGenerated:\n{}",
        rust_code
    );

    // String literals are auto-converted with .to_string()
    assert!(
        rust_code.contains(r#"greet("Alice".to_string())"#),
        "String literals should be converted to String with .to_string().\nGenerated:\n{}",
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

    let rust_code = compile_code(source).expect("Compilation failed");
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

    let rust_code = compile_code(source).expect("Compilation failed");
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

    let rust_code = compile_code(source).expect("Compilation failed");
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
