use std::path::PathBuf;
use std::process::Command;

/// Helper to compile Windjammer code and return the generated Rust code
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
fn test_push_str_with_string_variable() {
    let code = r#"
    pub fn build_html(display: string) -> string {
        let html = "<div>"
        html.push_str(display)
        html.push_str("</div>")
        return html
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // push_str expects &str, so String args should be borrowed with &
    assert!(
        generated.contains("push_str(&display)"),
        "push_str with String variable should auto-borrow: {}",
        generated
    );
    assert!(
        generated.contains(r#"push_str("</div>")"#),
        "push_str with string literal should not add &"
    );
}

#[test]
fn test_push_str_with_string_expression() {
    let code = r#"
    pub fn build_tag(tag: string) -> string {
        let html = "<"
        html.push_str(tag.clone())
        html.push_str(">")
        return html
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // tag.clone() returns String, needs & for push_str
    assert!(
        generated.contains("push_str(&tag.clone())"),
        "push_str with String expression should auto-borrow: {}",
        generated
    );
}

#[test]
fn test_push_str_with_conditional() {
    let code = r#"
    pub fn build_style(enabled: bool) -> string {
        let html = "style=\""
        let value = if enabled { "visible" } else { "hidden" }
        html.push_str(value)
        html.push_str("\"")
        return html
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // Note: if-else string literals may be converted to String for consistency,
    // in which case value needs & for push_str
    // This is correct behavior - push_str(&value) when value is String
    let has_correct_push_str =
        generated.contains("push_str(&value)") || generated.contains("push_str(value)");
    assert!(
        has_correct_push_str,
        "push_str should handle value correctly (with or without &): {}",
        generated
    );
}
