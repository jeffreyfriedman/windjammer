/// TDD Test: Extern function calls with string literal arguments expecting String
///
/// Bug: When calling an extern function that takes `string` (→ String in Rust),
/// passing a string literal like `"hello"` generates `"hello"` (&str) instead
/// of `"hello".to_string()` (String). This causes a type mismatch error:
///   error[E0308]: mismatched types -- expected `String`, found `&str`
///
/// Root Cause: The analyzer infers `Borrowed` ownership for extern function
/// parameters (since the body is empty), but the string-literal-to-String
/// conversion only fires for `Owned` ownership. For extern functions, the
/// parameter type (String) should take precedence over inferred ownership.
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn compile_windjammer_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let test_dir = temp_dir.path();
    let input_file = test_dir.join("test.wj");
    std::fs::write(&input_file, code).expect("Failed to write source file");

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
        .args([
            "build",
            input_file.to_str().unwrap(),
            "--output",
            test_dir.join("build").to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        return Err(format!(
            "Windjammer compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = test_dir.join("build/test.rs");
    let generated =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated file");
    Ok(generated)
}

#[test]
fn test_extern_fn_string_literal_gets_to_string() {
    let code = r#"
extern fn load_shader(path: string) -> u32

fn main() {
    let id = load_shader("shaders/test.wgsl")
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains(r#""shaders/test.wgsl".to_string()"#),
        "String literal passed to extern fn expecting String should get .to_string().\nGenerated code:\n{}",
        generated
    );
}

#[test]
fn test_extern_fn_string_literal_multiple_params() {
    let code = r#"
extern fn create_window(title: string, width: u32, height: u32)

fn main() {
    create_window("My Window", 800, 600)
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains(r#""My Window".to_string()"#),
        "String literal in first param should get .to_string().\nGenerated code:\n{}",
        generated
    );
}

#[test]
fn test_extern_fn_string_literal_variable_still_works() {
    let code = r#"
extern fn load_file(path: string) -> u32

fn main() {
    let p = "test.txt"
    let id = load_file(p)
}
"#;

    let generated = compile_windjammer_code(code).expect("Compilation should succeed");

    assert!(
        generated.contains("load_file("),
        "Should generate a call to load_file.\nGenerated code:\n{}",
        generated
    );
}
