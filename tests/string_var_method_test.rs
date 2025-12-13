//! TDD Test: String literal variables passed to methods expecting String
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_get_generated(code: &str) -> (bool, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");
    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");
    if !output.status.success() {
        return (false, String::new());
    }
    let generated_path = out_dir.join("test.rs");
    (
        true,
        fs::read_to_string(&generated_path).unwrap_or_default(),
    )
}

#[test]
fn test_string_literal_var_to_method() {
    let code = r#"
fn process(content: string) -> string { content }
fn render() -> string {
    let content = "<div>Hello</div>"
    process(content)
}
"#;
    let (success, generated) = compile_and_get_generated(code);
    println!("Generated:\n{}", generated);
    assert!(success, "Compilation should succeed");
    // content should be String, not &str
    let has_conversion = generated.contains(".to_string()");
    assert!(
        has_conversion,
        "String literal var should convert. Generated:\n{}",
        generated
    );
}
