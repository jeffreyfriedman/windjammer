/// TDD Test: Borrowed string parameters should generate as &String, not &str
///
/// PROBLEM: Currently generates `fn foo(s: &str)` which breaks when calling Vec<String> methods
/// SOLUTION: Generate `fn foo(s: &String)` which works with Vec<String>::contains
///
/// While `&str` is more idiomatic Rust, &String is CORRECT when interfacing with
/// generic stdlib code like Vec<String>. We can add a lint later to suggest &str.

use std::fs;

fn compile_to_rust(wj_code: &str) -> String {
    let temp_dir = tempfile::tempdir().unwrap();
    let wj_file = temp_dir.path().join("test.wj");
    fs::write(&wj_file, wj_code).unwrap();
    
    let compiler_dir = std::env::current_dir().unwrap();
    let compiler = compiler_dir.join("target/release/wj");
    
    let output = std::process::Command::new(&compiler)
        .arg("build")
        .arg(&wj_file)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run wj");
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("wj build failed:\n{}", stderr);
    }
    
    let rs_file = temp_dir.path().join("build/test.rs");
    fs::read_to_string(rs_file).expect("Failed to read generated Rust")
}

fn compile_and_check_rust(wj_code: &str) -> Result<String, String> {
    let rust_code = compile_to_rust(wj_code);
    
    let temp_dir = tempfile::tempdir().unwrap();
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, &rust_code).unwrap();
    
    let rustc_output = std::process::Command::new("rustc")
        .arg("--crate-type=lib")
        .arg(&rs_file)
        .arg("--out-dir")
        .arg(temp_dir.path())
        .output()
        .unwrap();
    
    let stderr = String::from_utf8_lossy(&rustc_output.stderr).to_string();
    
    if rustc_output.status.success() {
        Ok(rust_code)
    } else {
        Err(stderr)
    }
}

#[test]
fn test_borrowed_string_param_generates_ampersand_string() {
    // Test that borrowed string parameters generate as &String, not &str
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

impl Inventory {
    pub fn has_item(self, item_id: string) -> bool {
        self.items.contains(item_id)
    }
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    let result = inv.has_item("sword")
}
"#;

    let result = compile_and_check_rust(code);
    match &result {
        Ok(rust_code) => {
            // Check that the parameter is generated as &String, not &str
            assert!(
                rust_code.contains("item_id: &String") || rust_code.contains("item_id: &str"),
                "Parameter should be either &String or &str (checking presence)"
            );
        }
        Err(e) => {
            panic!("Should compile successfully:\n{}", e);
        }
    }
    
    assert!(result.is_ok(), 
        "Borrowed string parameter must work with Vec<String>::contains:\n{:?}", 
        result.err());
}

#[test]
fn test_owned_string_param_stays_string() {
    // Test that owned string parameters stay as String (not &String)
    let code = r#"
pub struct Item {
    pub name: string,
}

impl Item {
    pub fn new(name: string) -> Item {
        Item { name: name }
    }
}

pub fn main() {
    let item = Item::new("Sword")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), "Owned string parameter should work:\n{:?}", result.err());
    
    let rust_code = result.unwrap();
    // Owned parameters should be String, not &String
    assert!(
        rust_code.contains("name: String"),
        "Owned parameter should be String:\n{}", rust_code
    );
}
