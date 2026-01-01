//! TDD Test: Auto-clone iterator variable when pushing to Vec that expects owned
//!
//! When iterating over borrowed collection and pushing to a new Vec that will
//! be assigned to a field, we need to clone the iterator variable.

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
fn test_filter_push_clone() {
    // When filtering a Vec and pushing to new Vec, need to clone
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    id: i32,
    name: string,
}

pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn remove_item(&mut self, target_id: i32) {
        let mut new_items = Vec::new()
        for item in self.items {
            if item.id != target_id {
                new_items.push(item)
            }
        }
        self.items = new_items
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // The item should be cloned when pushed
    assert!(
        generated.contains("item.clone()") || generated.contains(".clone()"),
        "Should clone iterator variable. Generated:\n{}",
        generated
    );
    assert!(success, "Generated code should compile. Error: {}", err);
}
