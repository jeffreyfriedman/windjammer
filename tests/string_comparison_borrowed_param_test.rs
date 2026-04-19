/// TDD Test: String comparisons with borrowed &String parameters
///
/// PROBLEM: self.field == borrowed_param fails when field is String and param is &String
/// SOLUTION: Auto-add * deref for &String in comparisons, or use .as_str() for both sides
///
/// This test validates that String comparisons work correctly after changing
/// borrowed parameters from &str to &String

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
fn test_string_field_equals_borrowed_param() {
    // Test: self.field == param where field is String and param is &String
    let code = r#"
pub struct Player {
    pub name: string,
}

impl Player {
    pub fn has_name(self, n: string) -> bool {
        self.name == n
    }
}

pub fn main() {
    let p = Player { name: "Alice" }
    let result = p.has_name("Alice")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "String field == borrowed &String param should work:\n{:?}", 
        result.err());
}

#[test]
fn test_borrowed_param_equals_string_field() {
    // Test: param == self.field (reversed operands)
    let code = r#"
pub struct Player {
    pub name: string,
}

impl Player {
    pub fn has_name(self, n: string) -> bool {
        n == self.name
    }
}

pub fn main() {
    let p = Player { name: "Alice" }
    let result = p.has_name("Alice")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "Borrowed &String param == String field should work:\n{:?}", 
        result.err());
}

#[test]
fn test_string_comparison_in_complex_expression() {
    // Test: String comparisons in &&, || expressions
    let code = r#"
pub struct Player {
    pub name: string,
    pub title: string,
}

impl Player {
    pub fn matches(self, n: string, t: string) -> bool {
        self.name == n && self.title == t
    }
}

pub fn main() {
    let p = Player { name: "Alice", title: "Knight" }
    let result = p.matches("Alice", "Knight")
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "Multiple String comparisons should work:\n{:?}", 
        result.err());
}

#[test]
fn test_string_comparison_with_match_arm_binding() {
    // Test: Match arm binding (String) compared with field (String)
    let code = r#"
pub struct Inventory {
    pub items: Vec<string>,
}

pub enum Condition {
    HasItem(string),
}

impl Condition {
    pub fn check(self, inv: Inventory) -> bool {
        match self {
            Condition::HasItem(item_id) => {
                // This compares String (from match arm) with String (from Vec)
                inv.items.iter().any(|stored| stored == item_id)
            }
        }
    }
}

pub fn main() {
    let inv = Inventory { items: Vec::new() }
    let cond = Condition::HasItem("sword")
    let result = cond.check(inv)
}
"#;

    let result = compile_and_check_rust(code);
    assert!(result.is_ok(), 
        "Match arm binding comparison should work:\n{:?}", 
        result.err());
}
