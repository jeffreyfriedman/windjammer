/// TDD Tests for Phase 3 Parameter Decorators
///
/// These tests verify manual override decorators:
/// - @str_ref forces &str (developer promises it's safe)
/// - @string_ref forces &String (conservative override)
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    fs::write(&input_file, code).expect("Failed to write source file");

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
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
fn test_str_ref_decorator_forces_str() {
    // TDD: @str_ref forces &str even if function uses Vec<String> methods
    // NOTE: This will cause rustc E0308, but that's the developer's responsibility
    let code = r#"
pub fn log(@str_ref msg: string) {
    println!("{}", msg)
}

fn main() {
    log("Hello")
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 3: @str_ref forces &str
    assert!(
        generated.contains("fn log(msg: &str)"),
        "Expected @str_ref to force &str parameter. Generated:\n{}",
        generated
    );

    // String literal should pass directly (no conversion)
    assert!(
        generated.contains(r#"log("Hello")"#),
        "Expected direct string literal with &str param. Generated:\n{}",
        generated
    );
}

#[test]
fn test_string_ref_decorator_forces_string() {
    // TDD: @string_ref forces &String even if auto-optimization would choose &str
    let code = r#"
pub fn log(@string_ref msg: string) {
    println!("{}", msg)
}

fn main() {
    log("Hello")
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 3: @string_ref forces &String
    assert!(
        generated.contains("fn log(msg: &String)"),
        "Expected @string_ref to force &String parameter. Generated:\n{}",
        generated
    );

    // String literal must be converted
    assert!(
        generated.contains(r#"log(&"Hello".to_string())"#),
        "Expected converted string literal with &String param. Generated:\n{}",
        generated
    );
}

#[test]
fn test_mixed_decorated_and_inferred_params() {
    // TDD: Mix of decorated and inferred parameters
    let code = r#"
pub fn process(@str_ref fast: string, @string_ref safe: string, inferred: string) {
    println!("{} {} {}", fast, safe, inferred)
}

fn main() {
    process("a", "b", "c")
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 3: Decorators override, inferred uses Phase 2 automatic optimization
    // With Phase 2 enabled globally, non-decorated params are optimized to &str when safe
    assert!(
        generated.contains("fn process(fast: &str, safe: &String, inferred: &str)"),
        "Expected mixed decorators: fast=&str (decorator), safe=&String (decorator), inferred=&str (Phase 2 auto-opt). Generated:\n{}",
        generated
    );
}

#[test]
fn test_decorator_overrides_automatic_analysis() {
    // TDD: @str_ref overrides automatic analysis even when it would choose &String
    // This test will be relevant when automatic analysis is fully implemented
    let code = r#"
struct Store {
    items: Vec<string>
}

impl Store {
    // Developer promises this is safe (maybe they know Vec methods aren't called)
    pub fn check(@str_ref item_id: string) -> bool {
        // In reality, if this calls self.items.contains(item_id), rustc will error
        // But that's the developer's responsibility when using @str_ref
        item_id.len() > 0
    }
}

fn main() {
    let store = Store { items: Vec::new() }
    let result = store.check("test")
    println!("{}", result)
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 3: @str_ref forces &str (even as static method)
    // Note: Method defined without self, so it's a static method
    assert!(
        generated.contains("fn check(item_id: &str)"),
        "Expected @str_ref to force &str in static method. Generated:\n{}",
        generated
    );
}
