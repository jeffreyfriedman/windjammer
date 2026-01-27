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
fn test_struct_field_some_string_literal() {
    let code = r#"
    struct Config {
        name: string,
        parent: Option<string>,
    }
    impl Config {
        pub fn new() -> Config {
            Config {
                name: "default",
                parent: Some("root"),
            }
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // Some("root") should convert to Some("root".to_string())
    assert!(
        generated.contains(r#"Some("root".to_string())"#),
        "Some with string literal should auto-convert in struct field: {}",
        generated
    );
    assert!(
        generated.contains(r#"name: "default".to_string()"#),
        "Direct string literal field should also convert: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_field_ok_string_literal() {
    let code = r#"
    struct Response {
        status: Result<string, string>,
    }
    impl Response {
        pub fn success() -> Response {
            Response {
                status: Ok("success"),
            }
        }
    }
    "#;
    let generated = compile_code(code).expect("Compilation failed");
    // Ok("success") should convert to Ok("success".to_string())
    assert!(
        generated.contains(r#"Ok("success".to_string())"#),
        "Ok with string literal should auto-convert in struct field: {}",
        generated
    );
}
