//! TDD Test: Borrowed reference fields need .clone() when passed to methods
//!
//! When iterating over borrowed items and passing their fields to methods,
//! the fields need to be cloned since we can't move out of a reference.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

    // Use the pre-built wj binary directly (much faster than cargo run, especially under tarpaulin)
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
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
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_item_field_access() {
    // When iterating over borrowed items, fields need .clone()
    let code = r#"
pub struct Property {
    pub name: string,
    pub value: string,
}

pub fn process_property(name: string, value: string) -> string {
    format!("{}: {}", name, value)
}

pub fn process_properties(props: &Vec<Property>) -> string {
    let mut result = "".to_string()
    for prop in props {
        result = result + process_property(prop.name, prop.value).as_str()
    }
    result
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The prop.name and prop.value should have .clone() added
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_with_borrowed_fields() {
    // Method calls with borrowed item fields
    let code = r#"
pub struct Item {
    pub label: string,
    pub description: string,
}

pub struct Display {
    items: Vec<Item>,
}

impl Display {
    pub fn render_item(&self, label: string, description: string) -> string {
        format!("<div>{}: {}</div>", label, description)
    }
    
    pub fn render_all(&self) -> string {
        let mut result = "".to_string()
        for item in self.items {
            result = result + self.render_item(item.label, item.description).as_str()
        }
        result
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    assert!(
        success,
        "Method call with borrowed fields should compile. Error: {}",
        err
    );
}
