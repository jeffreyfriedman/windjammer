//! TDD Test: Struct constructor argument handling
//!
//! Tests that when calling struct constructors (like Node::new(name)),
//! the arguments are correctly converted based on the parameter ownership.

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
fn test_push_struct_with_borrowed_param() {
    // When a method has a borrowed string param and we use it to create a struct,
    // the struct constructor should receive the correct type
    let code = r#"
pub struct Node {
    id: string,
    name: string,
}

impl Node {
    pub fn new(id: string, name: string) -> Node {
        Node { id: id, name: name }
    }
}

pub struct Editor {
    nodes: Vec<Node>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor { nodes: Vec::new() }
    }
    
    // name is borrowed (only passed to struct constructor)
    pub fn add_node(&mut self, id: string, name: string) {
        self.nodes.push(Node::new(id, name))
    }
}

pub fn test_editor() {
    let mut editor = Editor::new()
    editor.add_node("1", "First")
    editor.add_node("2", "Second")
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
fn test_push_cloned_struct_to_vec() {
    // When iterating and pushing clones, the vec should work correctly
    // WINDJAMMER FIX: No need for .as_str() when parameter is already inferred to &str
    let code = r#"
pub struct Item {
    name: string,
}

pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: Vec::new() }
    }
    
    pub fn filter_items(&self, prefix: string) -> Vec<Item> {
        let mut result = Vec::new()
        for item in self.items.iter() {
            if item.name.starts_with(prefix) {
                result.push(item.clone())
            }
        }
        result
    }
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
fn test_constructor_with_multiple_string_params() {
    // Constructor with multiple string params that are stored
    let code = r#"
pub struct Person {
    first_name: string,
    last_name: string,
    email: string,
}

impl Person {
    pub fn new(first: string, last: string, email: string) -> Person {
        Person {
            first_name: first,
            last_name: last,
            email: email,
        }
    }
}

pub fn create_person() -> Person {
    Person::new("John", "Doe", "john@example.com")
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
fn test_method_call_chain_with_borrowed_params() {
    // Method that takes borrowed params and passes to another method
    let code = r#"
pub struct Logger {
    prefix: string,
}

impl Logger {
    pub fn new(prefix: string) -> Logger {
        Logger { prefix: prefix }
    }
    
    pub fn log(&self, message: string) {
        println!("[{}] {}", self.prefix, message)
    }
}

pub fn test_logger() {
    // Logger::new takes owned string, so "MyApp" should get .to_string()
    let logger = Logger::new("MyApp")
    logger.log("Application started")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    if !success {
        println!("Generated code:\n{}", generated);
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Generated code should compile. Error: {}", err);
}
