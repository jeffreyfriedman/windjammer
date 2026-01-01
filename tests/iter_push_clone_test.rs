//! TDD Test: Auto-clone when pushing iterator items to new Vec
//!
//! When iterating over self.field and pushing items to a new Vec,
//! the items should be cloned since the iterator returns references.

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
fn test_filter_self_items_to_new_vec() {
    // Common pattern: filter items from self and collect into new Vec
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    id: i32,
    name: string,
}

@derive(Clone, Debug)
pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: Vec::new() }
    }
    
    // Filter items and return a new Vec
    pub fn filter_by_id(&self, min_id: i32) -> Vec<Item> {
        let mut result = Vec::new()
        for item in self.items {
            if item.id >= min_id {
                result.push(item)
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

    // Should auto-clone items when pushing to result
    assert!(success, "Generated code should compile. Error: {}", err);
}

#[test]
fn test_remove_item_rebuild_vec() {
    // Pattern: rebuild vec without certain items
    let code = r#"
@derive(Clone, Debug)
pub struct Bus {
    name: string,
    volume: f32,
}

@derive(Clone, Debug)
pub struct Mixer {
    buses: Vec<Bus>,
}

impl Mixer {
    pub fn new() -> Mixer {
        Mixer { buses: Vec::new() }
    }
    
    // Remove bus at index by rebuilding Vec
    pub fn remove_bus(&mut self, index: i32) {
        let mut new_buses = Vec::new()
        let mut i = 0
        for bus in self.buses {
            if i != index {
                new_buses.push(bus)
            }
            i = i + 1
        }
        self.buses = new_buses
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
fn test_copy_matching_items() {
    // Copy items that match a condition
    let code = r#"
@derive(Clone, Debug, Copy)
pub struct Node {
    id: i32,
    parent_id: i32,
}

@derive(Clone, Debug)
pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    pub fn new() -> Tree {
        Tree { nodes: Vec::new() }
    }
    
    pub fn get_children(&self, parent: i32) -> Vec<Node> {
        let mut children = Vec::new()
        for node in self.nodes {
            if node.parent_id == parent {
                children.push(node)
            }
        }
        children
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
