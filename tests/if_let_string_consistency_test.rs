use std::path::PathBuf;
use std::process::Command;

fn compile_code(code: &str) -> Result<String, String> {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let src_file = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::create_dir(&out_dir).map_err(|e| format!("Failed to create out dir: {}", e))?;

    let mut file =
        fs::File::create(&src_file).map_err(|e| format!("Failed to create source file: {}", e))?;
    file.write_all(code.as_bytes())
        .map_err(|e| format!("Failed to write source: {}", e))?;

    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");

    let output = Command::new(&wj_binary)
        .arg("build")
        .arg(&src_file)
        .arg("-o")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = out_dir.join("test.rs");
    fs::read_to_string(&generated_file).map_err(|e| format!("Failed to read generated file: {}", e))
}

#[test]
fn test_if_let_string_literal_consistency() {
    let code = r#"
    fn classify(x: Option<i32>) -> string {
        let result = if let Some(val) = x {
            if val > 0 { "positive" } else { "negative" }
        } else {
            "none"
        }
        result
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // When if-let is transformed to match, all arms should return consistent types
    // If Some arm returns String (via .to_string()), None arm should too
    assert!(
        generated.contains(r#""none".to_string()"#)
            || !generated.contains(r#""positive".to_string()"#),
        "Match arms should have consistent types: {}",
        generated
    );
}

#[test]
fn test_if_let_with_function_return() {
    let code = r#"
    fn get_status(active: Option<i32>) -> string {
        if let Some(id) = active {
            if id == 1 { "active" } else { "inactive" }
        } else {
            "unknown"
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // All branches should return String consistently
    println!("Generated:\n{}", generated);
}

