/// TDD Test: Owned parameters assigned to struct fields should stay owned
///
/// When a non-Copy parameter is directly assigned to a struct field,
/// it should NOT be inferred as borrowed (&T), it should stay owned (T).
///
/// This follows "The Windjammer Way" - the compiler does the work.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_vec_param_assigned_to_field_stays_owned() {
    let code = r#"
pub struct Node {
    pub items: Vec<string>,
}

impl Node {
    pub fn with_items(items: Vec<string>) -> Node {
        let mut node = Node { items: Vec::new() }
        node.items = items
        node
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .current_dir(output_dir)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "Compilation failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Read generated Rust code
    let generated = fs::read_to_string(output_dir.join("test.rs")).unwrap();
    println!("Generated:\n{}", generated);

    // The parameter should stay owned: items: Vec<String>
    // NOT borrowed: items: &Vec<String>
    assert!(
        generated.contains("pub fn with_items(items: Vec<String>)"),
        "Expected 'items: Vec<String>' (owned), not borrowed, got:\n{}",
        generated
    );

    // Should not have &Vec in the function signature
    assert!(
        !generated.contains("pub fn with_items(items: &Vec<String>)"),
        "Parameter should NOT be borrowed when assigned to field, got:\n{}",
        generated
    );
}

#[test]
fn test_owned_param_in_struct_literal_stays_owned() {
    let code = r#"
pub struct Config {
    pub items: Vec<string>,
}

impl Config {
    pub fn new(items: Vec<string>) -> Config {
        Config { items }
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path();
    let wj_file = output_dir.join("test.wj");

    fs::write(&wj_file, code).unwrap();

    // Compile
    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("build")
        .arg(&wj_file)
        .arg("--output")
        .arg(output_dir)
        .arg("--no-cargo")
        .current_dir(output_dir)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "Compilation failed:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );

    // Read generated Rust code
    let generated = fs::read_to_string(output_dir.join("test.rs")).unwrap();
    println!("Generated:\n{}", generated);

    // The parameter should stay owned
    assert!(
        generated.contains("pub fn new(items: Vec<String>)"),
        "Expected 'items: Vec<String>' (owned), got:\n{}",
        generated
    );
}
