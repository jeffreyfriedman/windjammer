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
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_ref_field_vs_literal() {
    let code = r###"
    struct Rating {
        color: string,
    }
    impl Rating {
        pub fn get_color(&self, filled: bool) -> &str {
            if filled {
                &self.color
            } else {
                "#e2e8f0"
            }
        }
    }
    "###;
    let generated = compile_code(code).expect("Compilation failed");
    // When one branch is &self.field (explicit ref) and other is string literal,
    // the literal should NOT be converted to String
    // Both are &str compatible
    assert!(
        !generated.contains(r###""#e2e8f0".to_string()"###),
        "String literal should NOT be converted when other branch is explicit &ref: {}",
        generated
    );
    assert!(
        generated.contains(r###""#e2e8f0""###),
        "String literal should remain as &str: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_if_else_ref_vs_literal_in_let() {
    let code = r#"
    struct Config {
        name: string,
    }
    pub fn get_display_name(config: &Config, use_default: bool) -> &str {
        let name = if use_default {
            &config.name
        } else {
            "Unknown"
        }
        return name
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // String literal should stay as &str to match &String
    assert!(
        !generated.contains(r#""Unknown".to_string()"#),
        "String literal should NOT be converted when other branch is &field: {}",
        generated
    );
}
