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
fn test_void_return_preserves_semicolon() {
    let code = r#"
    use std::collections::HashMap;
    
    struct Store {
        items: HashMap<string, i32>,
    }
    impl Store {
        pub fn add(&mut self, key: string, value: i32) {
            self.items.insert(key, value);
        }
        
        pub fn remove(&mut self, key: &string) {
            self.items.remove(key);
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // The semicolons should be preserved to discard the return value
    assert!(
        generated.contains("insert(key, value);"),
        "insert() should end with semicolon: {}",
        generated
    );
    // key is already &string in the parameter, so we don't add another &
    // We just need to verify the semicolon is preserved
    assert!(
        generated.contains("remove(key);"),
        "remove() should end with semicolon: {}",
        generated
    );
}
