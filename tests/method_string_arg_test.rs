//! TDD Test: Method call string arguments need .to_string()
//!
//! When calling a method like `.icon("📄")` where the method expects String,
//! the string literal should be converted to String automatically.

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
fn test_method_chain_string_args() {
    // Method chain with string literal arguments
    let code = r#"
pub struct MenuItem {
    label: string,
    icon: string,
}

impl MenuItem {
    pub fn new(label: string) -> MenuItem {
        MenuItem { label: label, icon: "".to_string() }
    }
    
    pub fn icon(self, icon: string) -> MenuItem {
        MenuItem { label: self.label, icon: icon }
    }
}

pub fn create_menu() -> MenuItem {
    MenuItem::new("File").icon("📁")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The .icon("📁") should have .to_string() added
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
fn test_builder_pattern_string_args() {
    // Builder pattern with multiple string methods
    let code = r#"
pub struct Config {
    name: string,
    description: string,
    author: string,
}

impl Config {
    pub fn new() -> Config {
        Config { 
            name: "".to_string(), 
            description: "".to_string(),
            author: "".to_string(),
        }
    }
    
    pub fn name(self, name: string) -> Config {
        Config { name: name, description: self.description, author: self.author }
    }
    
    pub fn description(self, desc: string) -> Config {
        Config { name: self.name, description: desc, author: self.author }
    }
    
    pub fn author(self, author: string) -> Config {
        Config { name: self.name, description: self.description, author: author }
    }
}

pub fn create_config() -> Config {
    Config::new()
        .name("MyApp")
        .description("A great app")
        .author("Me")
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(success, "Builder pattern should compile. Error: {}", err);
}

#[test]
fn test_vec_push_string_literal() {
    // Vec::push with string literal
    let code = r#"
pub fn test_push() -> Vec<string> {
    let mut items = Vec::new()
    items.push("first")
    items.push("second")
    items
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Vec push should work with string literals. Error: {}",
        err
    );
}
