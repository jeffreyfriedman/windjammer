/// TDD Test: String Ownership in Method Calls
///
/// Bug: Compiler adds `&` when passing String to method expecting owned String
/// Expected: String variables should be passed as-is (moved)
/// Actual: Compiler adds unnecessary `&String`
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
fn test_string_passed_owned_to_method() {
    let source = r#"
struct Renderer {}

impl Renderer {
    fn draw_text(self, text: string) {
        println!("{}", text)
    }
}

fn main() {
    let renderer = Renderer{}
    let message = "Hello".to_string()
    renderer.draw_text(message)  // Should pass owned, not &message
}
"#;

    let rust_code = compile_code(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: String should be moved, not borrowed
    // Should generate: renderer.draw_text(message)
    // NOT: renderer.draw_text(&message)
    assert!(
        rust_code.contains("renderer.draw_text(message)"),
        "String variable should be passed owned (moved), not as &.\nGenerated:\n{}",
        rust_code
    );

    // Should NOT contain &message
    assert!(
        !rust_code.contains("renderer.draw_text(&message)"),
        "Should NOT add & when method expects owned String"
    );
}

#[test]
fn test_string_literal_converted_to_string() {
    let source = r#"
struct Renderer {}

impl Renderer {
    fn draw_text(&self, text: string) {
        println!("{}", text)
    }
}

fn main() {
    let renderer = Renderer{}
    renderer.draw_text("Hello World")  // Should convert &str to String
}
"#;

    let rust_code = compile_code(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: String literals should be auto-converted to String
    // Should generate: renderer.draw_text("Hello World".to_string())
    // OR: renderer.draw_text(String::from("Hello World"))
    let has_to_string = rust_code.contains(r#""Hello World".to_string()"#);
    let has_string_from = rust_code.contains(r#"String::from("Hello World")"#);

    assert!(
        has_to_string || has_string_from,
        "String literal should be auto-converted to String when method expects owned String.\nGenerated:\n{}",
        rust_code
    );
}
