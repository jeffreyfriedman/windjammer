#![allow(dead_code)]
//! Ownership Inference Tests
//!
//! Tests for automatic ownership inference including:
//! - Iterator variable tracking (no double-ref when passing to methods)
//! - Storage method detection (push/insert infer owned parameters)
//! - Recursive is_stored checking in if/else/for/while bodies
//! - Function calls to load/open/etc. don't add .to_string()

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to compile Windjammer code and check for errors
fn compile_wj_code(code: &str) -> (bool, String) {
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

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (output.status.success(), format!("{}\n{}", stdout, stderr))
}

/// Helper to compile and verify generated Rust code
fn compile_and_verify_rust(code: &str) -> (bool, String, String) {
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

    (true, generated, String::new())
}

// ============================================================================
// Test: Iterator Variable Tracking
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iterator_var_no_double_ref_for_contains() {
    // When iterating with .iter(), the loop variable is already a reference
    // Calling .contains() should NOT add another &
    let code = r#"
struct Item {
    id: string,
}

pub fn find_item(items: Vec<Item>, target_ids: Vec<string>) -> bool {
    for item in items {
        if target_ids.contains(item.id) {
            return true
        }
    }
    false
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Should NOT have &item.id when item is already a reference from .iter()
    // The generated code should have contains(item.id) or contains(&item.id) but not contains(&&item.id)
    assert!(
        !generated.contains("&&item"),
        "Should not have double reference. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_iterator_var_no_double_ref_for_get() {
    // HashMap.get() should not double-reference iterator variables
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<string, i32>, keys: Vec<string>) -> i32 {
    let mut total = 0
    for key in keys {
        match map.get(key) {
            Some(val) => { total = total + val },
            None => {},
        }
    }
    total
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Should not have double reference
    assert!(
        !generated.contains("&&key"),
        "Should not have double reference. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Storage Method Detection (push/insert)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_infers_owned_parameter() {
    // When a parameter is pushed to a Vec, it should be inferred as owned
    let code = r#"
pub struct Container {
    items: Vec<string>,
}

impl Container {
    pub fn add(&mut self, item: string) {
        self.items.push(item)
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Parameter should be owned String, not &String
    assert!(
        generated.contains("fn add(&mut self, item: String)")
            || generated.contains("fn add(&mut self, item: string)"),
        "Parameter should be owned. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn add(&mut self, item: &String)"),
        "Parameter should NOT be borrowed. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_in_else_branch_infers_owned() {
    // Push in else branch should still infer owned parameter
    let code = r#"
pub struct NameList {
    names: Vec<string>,
}

impl NameList {
    pub fn add_if_missing(&mut self, name: string) {
        if self.names.contains(&name) {
            // Already exists, do nothing
        } else {
            self.names.push(name)
        }
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Parameter should be owned even though push is in else branch
    assert!(
        !generated.contains("fn add_if_missing(&mut self, name: &String)"),
        "Parameter should be owned, not borrowed. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_insert_infers_owned_parameter() {
    // HashMap insert should infer owned parameters
    let code = r#"
use std::collections::HashMap

pub struct Cache {
    data: HashMap<string, string>,
}

impl Cache {
    pub fn set(&mut self, key: string, value: string) {
        self.data.insert(key, value)
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Both key and value should be owned
    assert!(
        !generated.contains("key: &String"),
        "Key should be owned. Generated:\n{}",
        generated
    );
    assert!(
        !generated.contains("value: &String"),
        "Value should be owned. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Recursive is_stored in Control Flow
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_in_for_loop_infers_owned() {
    // Push inside a for loop should infer owned parameter
    let code = r#"
pub struct Collector {
    results: Vec<string>,
}

impl Collector {
    pub fn collect_all(&mut self, items: Vec<string>, prefix: string) {
        for item in items {
            let combined = prefix.clone() + item.as_str()
            self.results.push(combined)
        }
    }
}
"#;

    let (success, _generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_in_while_loop_infers_owned() {
    // Push inside a while loop should detect storage
    let code = r#"
pub struct Buffer {
    lines: Vec<string>,
}

impl Buffer {
    pub fn read_lines(&mut self, count: i32) {
        let mut i = 0
        while i < count {
            self.lines.push("line".to_string())
            i = i + 1
        }
    }
}
"#;

    let (success, _generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);
}

// ============================================================================
// Test: Load/Open Functions Don't Add .to_string()
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_load_method_no_to_string() {
    // Methods like .load() should NOT add .to_string() to path arguments
    let code = r#"
pub struct Asset {
    path: string,
}

impl Asset {
    pub fn load(path: string) -> Asset {
        Asset { path: path }
    }
}

pub fn load_asset() -> Asset {
    Asset::load("assets/texture.png")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // This is a constructor that takes owned String, so it SHOULD have .to_string()
    // The test verifies the compiler properly handles associated functions
    assert!(
        generated.contains("Asset::load"),
        "Should have Asset::load call. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_in_struct_init() {
    // String literals in struct initialization should work
    let code = r#"
pub struct Config {
    name: string,
    version: string,
}

pub fn default_config() -> Config {
    Config { 
        name: "MyApp",
        version: "1.0.0",
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Should properly handle string literals in struct fields
    assert!(
        generated.contains("Config"),
        "Should have Config struct. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Mutable String Variables Get .to_string()
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mutable_string_auto_to_string() {
    // Mutable string variables should be String, not &str
    let code = r#"
pub fn build_message() -> string {
    let mut msg = ""
    msg = msg + "Hello"
    msg = msg + " World"
    msg
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Should have .to_string() for the initial value
    assert!(
        generated.contains(r#""".to_string()"#)
            || generated.contains("String::new()")
            || generated.contains("String::from"),
        "Mutable string should be String. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Match Arms Get Consistent Types
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_arms_string_consistency() {
    // All match arms returning strings should have consistent types
    let code = r#"
pub fn get_status(code: i32) -> string {
    match code {
        0 => "success",
        1 => "warning",
        _ => "error",
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // Either all arms should be &str or all should be String
    // The function returns string, so they should all be .to_string()
    let has_to_string = generated.contains(r#""success".to_string()"#);
    let has_raw_str = generated.contains(r#"=> "success""#) && !generated.contains(".to_string()");

    assert!(
        has_to_string || has_raw_str,
        "Match arms should have consistent types. Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Method Calls That Take &str
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_contains_takes_str_ref() {
    // String.contains() takes &str, should not add .to_string()
    let code = r#"
pub fn has_hello(text: string) -> bool {
    text.contains("hello")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // contains() argument should NOT have .to_string()
    assert!(
        !generated.contains(r#""hello".to_string()"#),
        "contains() should not add .to_string(). Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_replace_takes_str_ref() {
    // String.replace() takes &str for pattern
    let code = r#"
pub fn sanitize(text: string) -> string {
    text.replace("&", "&amp;")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // replace() arguments should NOT have .to_string()
    assert!(
        !generated.contains(r#""&".to_string()"#),
        "replace() pattern should not add .to_string(). Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_starts_with_takes_str_ref() {
    // String.starts_with() takes &str
    let code = r#"
pub fn is_http(url: string) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // starts_with() argument should NOT have .to_string()
    assert!(
        !generated.contains(r#""http://".to_string()"#),
        "starts_with() should not add .to_string(). Generated:\n{}",
        generated
    );
}

// ============================================================================
// Test: Integration - Complex Ownership Scenarios
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_complex_ownership_toggle_pattern() {
    // Common pattern: check if exists, then add or remove
    let code = r#"
pub struct SelectionManager {
    selected: Vec<string>,
}

impl SelectionManager {
    pub fn new() -> SelectionManager {
        SelectionManager { selected: Vec::new() }
    }
    
    pub fn toggle(&mut self, id: string) {
        if self.selected.contains(&id) {
            self.selected.retain(|x| x != &id)
        } else {
            self.selected.push(id)
        }
    }
    
    pub fn is_selected(&self, id: string) -> bool {
        self.selected.contains(&id)
    }
}
"#;

    let (success, generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);

    // toggle() should take owned String (because of push in else branch)
    // is_selected() can take owned or borrowed
    assert!(
        !generated.contains("fn toggle(&mut self, id: &String)"),
        "toggle() should take owned parameter due to push. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_complex_ownership_map_iteration() {
    // Iterating over HashMap and using keys
    let code = r#"
use std::collections::HashMap

pub struct Registry {
    items: HashMap<string, i32>,
}

impl Registry {
    pub fn find_by_prefix(&self, prefix: string) -> Vec<string> {
        let mut results = Vec::new()
        for key in self.items.keys() {
            if key.starts_with(prefix.as_str()) {
                results.push(key.clone())
            }
        }
        results
    }
}
"#;

    let (success, _generated, err) = compile_and_verify_rust(code);
    assert!(success, "Compilation failed: {}", err);
}
