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
fn test_iter_var_method_call() {
    let code = r#"
    pub fn process_strings(items: Vec<string>) -> Vec<bool> {
        let mut results = Vec::new()
        for item in items {
            let matches = item.as_str() == "test"
            results.push(matches)
        }
        return results
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // item.as_str() should be generated correctly, not item.as_str.clone()()
    assert!(
        generated.contains("item.as_str()"),
        "Method call should be item.as_str(): {}",
        generated
    );
    assert!(
        !generated.contains("item.as_str.clone()()"),
        "Should NOT have .clone() between method name and (): {}",
        generated
    );
}

#[test]
fn test_iter_var_method_call_in_comparison() {
    let code = r#"
    struct ThemeSwitcher {
        themes: Vec<string>,
        current_theme: string,
    }
    impl ThemeSwitcher {
        pub fn render(self) -> string {
            let mut output = "".to_string()
            for t in self.themes {
                let selected = if t.as_str() == self.current_theme.as_str() { "selected" } else { "" }
                output.push_str(selected)
            }
            return output
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    eprintln!("Generated code:\n{}", generated);
    // t.as_str() should be generated correctly
    assert!(
        generated.contains("t.as_str()"),
        "Method call should be t.as_str(): {}",
        generated
    );
    assert!(
        !generated.contains("t.as_str.clone()"),
        "Should NOT insert .clone() in method call: {}",
        generated
    );
}
