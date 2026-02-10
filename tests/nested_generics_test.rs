/// TDD Test: Nested Generics Parser Fix
///
/// Tests that the parser can handle nested generic types like HashMap<K, Vec<V>>
/// where the >> at the end should be treated as two > tokens.
use std::path::PathBuf;
use tempfile::TempDir;

fn compile_wj_to_rust(code: &str) -> Result<String, String> {
    use std::fs;
    use std::process::Command;

    // Create temp directory for test
    let temp_dir = TempDir::new().map_err(|e| e.to_string())?;
    let test_file = temp_dir.path().join("test.wj");

    // Write Windjammer code
    fs::write(&test_file, code).map_err(|e| e.to_string())?;

    // Compile with windjammer compiler
    let wj_binary = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = Command::new(&wj_binary)
        .arg("build")
        .arg("--no-cargo")
        .arg("test.wj")
        .current_dir(temp_dir.path())
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Read the generated Rust code (same name as input .wj file, but with .rs extension)
    let rust_file = temp_dir.path().join("build").join("test.rs");
    let rust_code = fs::read_to_string(rust_file).map_err(|e| e.to_string())?;

    Ok(rust_code)
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_generics_hashmap_vec() {
    let code = r#"
use std::collections::HashMap

pub struct Test {
    // This should parse correctly: HashMap<i64, Vec<i64>>
    // The >> should be treated as two > tokens
    map: HashMap<i64, Vec<i64>>,
}

impl Test {
    pub fn new() -> Test {
        Test {
            map: HashMap::new(),
        }
    }
}

fn main() {
    let test = Test::new()
    println!("Created test")
}
"#;

    let rust_code = compile_wj_to_rust(code).expect("Should compile");

    // Verify the generated Rust code contains the nested generic type
    assert!(
        rust_code.contains("HashMap<i64, Vec<i64>>"),
        "Generated Rust should contain HashMap<i64, Vec<i64>>"
    );
    assert!(
        rust_code.contains("pub struct Test"),
        "Generated Rust should contain struct Test"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_generics_option_vec() {
    let code = r#"
pub struct Test {
    opt: Option<Vec<i64>>,
}

impl Test {
    pub fn new() -> Test {
        Test {
            opt: None,
        }
    }
}

fn main() {
    let test = Test::new()
    println!("Created test")
}
"#;

    let rust_code = compile_wj_to_rust(code).expect("Should compile");

    // Verify the generated Rust code contains the nested generic type
    assert!(
        rust_code.contains("Option<Vec<i64>>"),
        "Generated Rust should contain Option<Vec<i64>>"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_triple_nested_generics() {
    let code = r#"
use std::collections::HashMap

pub struct Test {
    // Triple nesting: HashMap<i64, Option<Vec<i64>>>
    // This has >>> which should be treated as three > tokens
    map: HashMap<i64, Option<Vec<i64>>>,
}

impl Test {
    pub fn new() -> Test {
        Test {
            map: HashMap::new(),
        }
    }
}

fn main() {
    let test = Test::new()
    println!("Created test")
}
"#;

    let rust_code = compile_wj_to_rust(code).expect("Should compile");

    // Verify the generated Rust code contains the triple nested generic type
    assert!(
        rust_code.contains("HashMap<i64, Option<Vec<i64>>>"),
        "Generated Rust should contain HashMap<i64, Option<Vec<i64>>>"
    );
}
