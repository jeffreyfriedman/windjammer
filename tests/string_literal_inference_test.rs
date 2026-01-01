// Test: String literal inference in various contexts
// The compiler should automatically convert "literal" to String when context expects String

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
        .map_err(|e| format!("Failed to execute wj: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Compilation failed:\n{}", stderr));
    }

    let generated_file = out_dir.join("test.rs");
    fs::read_to_string(&generated_file).map_err(|e| format!("Failed to read generated file: {}", e))
}

#[test]
fn test_match_arm_string_inference() {
    let code = r#"
        fn get_status(opt: Option<i32>) -> string {
            match opt {
                Some(x) => "has value",
                None => "empty",
            }
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Match arms should infer .to_string() for string literals"
    );

    let generated = result.unwrap();

    // Should generate .to_string() automatically
    assert!(
        generated.contains("\"has value\".to_string()"),
        "Expected match arm to convert string literal to String"
    );
    assert!(
        generated.contains("\"empty\".to_string()"),
        "Expected match arm to convert string literal to String"
    );
}

#[test]
fn test_nested_match_string_inference() {
    let code = r#"
        fn get_class(selected: Option<string>, id: string) -> string {
            match selected {
                Some(sel_id) => if sel_id == id { "selected" } else { "normal" },
                None => "normal",
            }
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Nested if-else in match should infer .to_string()"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"selected\".to_string()"));
    assert!(generated.contains("\"normal\".to_string()"));
}

#[test]
fn test_if_else_string_inference() {
    let code = r#"
        fn get_status(is_active: bool) -> string {
            if is_active { "active" } else { "inactive" }
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "If-else should infer .to_string() when returning string"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"active\".to_string()"));
    assert!(generated.contains("\"inactive\".to_string()"));
}

#[test]
fn test_struct_field_string_inference() {
    let code = r#"
        struct Config {
            name: string,
            parent: Option<string>,
        }
        
        fn new_config() -> Config {
            Config {
                name: "default",
                parent: Some("root"),
            }
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Struct fields should infer .to_string() for string literals"
    );

    let generated = result.unwrap();
    assert!(generated.contains("name: \"default\".to_string()"));
    assert!(generated.contains("Some(\"root\".to_string())"));
}

#[test]
fn test_option_some_string_inference() {
    let code = r#"
        fn get_parent() -> Option<string> {
            Some("parent_id")
        }
        
        fn get_none() -> Option<string> {
            None
        }
    "#;

    let result = compile_code(code);
    assert!(result.is_ok(), "Option::Some should infer .to_string()");

    let generated = result.unwrap();
    assert!(generated.contains("Some(\"parent_id\".to_string())"));
}

#[test]
fn test_result_string_inference() {
    let code = r#"
        fn validate(value: i32) -> Result<string, string> {
            if value > 0 {
                Ok("valid")
            } else {
                Err("invalid")
            }
        }
    "#;

    let result = compile_code(code);
    assert!(result.is_ok(), "Result Ok/Err should infer .to_string()");

    let generated = result.unwrap();
    assert!(generated.contains("Ok(\"valid\".to_string())"));
    assert!(generated.contains("Err(\"invalid\".to_string())"));
}

#[test]
fn test_ternary_like_match_string_inference() {
    let code = r#"
        fn get_label(is_root: bool) -> string {
            if is_root { "🌟 Root" } else { "" }
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Ternary-like if-else should infer .to_string()"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"🌟 Root\".to_string()"));
    assert!(generated.contains("\"\".to_string()"));
}

#[test]
fn test_match_with_blocks_string_inference() {
    let code = r#"
        fn process(value: Option<i32>) -> string {
            match value {
                Some(x) => {
                    if x > 10 {
                        "large"
                    } else {
                        "small"
                    }
                },
                None => "empty",
            }
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Match arms with blocks should infer .to_string()"
    );

    let generated = result.unwrap();
    assert!(generated.contains("\"large\".to_string()"));
    assert!(generated.contains("\"small\".to_string()"));
    assert!(generated.contains("\"empty\".to_string()"));
}

#[test]
fn test_no_inference_for_str_return() {
    let code = r#"
        fn get_static() -> &str {
            "static"
        }
    "#;

    let result = compile_code(code);
    assert!(
        result.is_ok(),
        "Should NOT infer .to_string() when returning &str"
    );

    let generated = result.unwrap();
    assert!(
        !generated.contains("\"static\".to_string()"),
        "Should not convert to String when &str is expected"
    );
    assert!(generated.contains("\"static\""));
}
