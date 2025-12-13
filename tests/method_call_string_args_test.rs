//! TDD Test: Method call string argument handling
//!
//! This test verifies that when a method takes a string parameter that is only read,
//! and we call it with a string literal, the generated code compiles correctly.
//!
//! Issue: Method infers &String but caller adds .to_string() creating String

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run compiler");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return (false, String::new(), stderr);
    }

    let generated_path = out_dir.join("test.rs");
    let generated = fs::read_to_string(&generated_path)
        .unwrap_or_else(|_| "Failed to read generated file".to_string());

    // Try to compile the generated Rust code
    let rustc_output = Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&generated_path)
        .arg("-o")
        .arg(temp_dir.path().join("test.rlib"))
        .output();

    match rustc_output {
        Ok(output) => {
            let rustc_success = output.status.success();
            let rustc_err = String::from_utf8_lossy(&output.stderr).to_string();
            (rustc_success, generated, rustc_err)
        }
        Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
    }
}

#[test]
#[ignore] // TODO: Implement auto-.to_string() for method string arguments
fn test_method_with_read_only_string_param() {
    // A method that only reads its string parameter should work when called with literals
    let code = r#"
pub struct Editor {
    items: Vec<string>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor { items: Vec::new() }
    }
    
    // This method only reads 'name' - should infer correctly
    pub fn add_item(&mut self, name: string) -> i32 {
        println!("Adding: {}", name)
        self.items.len() as i32
    }
}

pub fn create_editor() {
    let mut editor = Editor::new()
    // These calls should work without type errors
    let _ = editor.add_item("First")
    let _ = editor.add_item("Second")
    let _ = editor.add_item("Third")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // Print debug info
    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[ignore] // TODO: Implement auto-.to_string() for method string arguments
fn test_method_with_stored_string_param() {
    // A method that stores its string parameter should take owned String
    let code = r#"
pub struct NameList {
    names: Vec<string>,
}

impl NameList {
    pub fn new() -> NameList {
        NameList { names: Vec::new() }
    }
    
    // This method stores 'name' - should be owned
    pub fn add(&mut self, name: string) {
        self.names.push(name)
    }
}

pub fn test_name_list() {
    let mut list = NameList::new()
    list.add("Alice")
    list.add("Bob")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[ignore] // TODO: Implement auto-.to_string() for method string arguments
fn test_method_returning_computed_value() {
    // A method that uses string for computation should handle correctly
    let code = r#"
pub struct Counter {
    count: i32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { count: 0 }
    }
    
    // Uses name for logging, returns computed value
    pub fn increment(&mut self, label: string) -> i32 {
        self.count = self.count + 1
        println!("{}: {}", label, self.count)
        self.count
    }
}

pub fn test_counter() {
    let mut counter = Counter::new()
    let a = counter.increment("Step A")
    let b = counter.increment("Step B")
    println!("Total: {}", a + b)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[ignore] // TODO: Implement auto-.to_string() for method string arguments
fn test_chained_method_calls_with_strings() {
    // Chained method calls with string parameters
    let code = r#"
pub struct Builder {
    parts: Vec<string>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder { parts: Vec::new() }
    }
    
    pub fn add(&mut self, part: string) -> &mut Builder {
        self.parts.push(part)
        self
    }
    
    pub fn build(&self) -> string {
        self.parts.join(", ")
    }
}

pub fn test_builder() -> string {
    let mut builder = Builder::new()
    builder.add("one").add("two").add("three")
    builder.build()
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}
