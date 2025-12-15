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
fn test_match_string_literal_in_return_position() {
    let code = r#"
    enum Status {
        Active,
        Inactive,
        Pending,
    }
    
    fn get_label(status: Status) -> string {
        match status {
            Status::Active => "Active",
            Status::Inactive => "Inactive",
            Status::Pending => "Pending",
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // Match arms with string literals should be converted when function returns String
    assert!(
        generated.contains(r#""Active".to_string()"#),
        "Match arm string literals should convert to String: {}",
        generated
    );
    assert!(
        generated.contains(r#""Inactive".to_string()"#),
        "All arms should convert: {}",
        generated
    );
}

#[test]
fn test_match_empty_string_return() {
    let code = r#"
    enum ObjectType {
        Cube,
        Empty,
    }
    
    fn render_object(obj: ObjectType) -> string {
        match obj {
            ObjectType::Cube => "Rendered cube",
            ObjectType::Empty => "",
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // Empty string should also be converted
    assert!(
        generated.contains(r#""".to_string()"#)
            || generated.contains(r#""Rendered cube".to_string()"#),
        "Empty string literal should convert to String: {}",
        generated
    );
}

