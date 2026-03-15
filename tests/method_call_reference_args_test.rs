//! TDD Test: Fix String: Borrow<&str> double-reference errors
//!
//! Bug: When calling HashMap.get() with key: str (which generates &str in Rust),
//! the codegen adds &, producing get(&key) = &&str, causing "String: Borrow<&str>" errors.
//!
//! Root cause: Codegen adds & to arguments that are already references.
//! Fix: Don't add & when arg type is already a reference (&str, &String, etc.)

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_verify(code: &str) -> (bool, String, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let wj_path = temp_dir.path().join("test.wj");
    let out_dir = temp_dir.path().join("out");

    fs::write(&wj_path, code).expect("Failed to write test file");
    fs::create_dir_all(&out_dir).expect("Failed to create output dir");

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

/// Case 1: hashmap.get(key) where key: str → generates hashmap.get(key) (no extra &)
/// str param becomes &str in Rust, so key is already a reference
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_str_param_no_double_ref() {
    let code = r#"
use std::collections::HashMap

pub fn get_data_int(map: HashMap<string, i32>, key: str) -> Option<i32> {
    match map.get(key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // key: str generates key: &str in Rust - must NOT add & (would create &&str)
    assert!(
        !generated.contains("map.get(&key)"),
        "Must not double-reference: key is already &str. Got: {}",
        generated
    );
    assert!(
        generated.contains("map.get(key)"),
        "Should use key directly when key: str. Got: {}",
        generated
    );
    assert!(success, "Generated Rust must compile. Error:\n{}", err);
}

/// Case 2: hashmap.get(key) where key: string (owned) → generates hashmap.get(&key) (add &)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_owned_string_param_adds_ref() {
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<string, i32>, key: string) -> Option<i32> {
    match map.get(key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // key: string (owned) needs & for HashMap.get(&K)
    assert!(
        generated.contains("map.get(&key)") || generated.contains("map.get(key)"),
        "Owned String key should work. Got: {}",
        generated
    );
    assert!(success, "Generated Rust must compile. Error:\n{}", err);
}

/// Case 3: vec.push(item) where item: T → generates vec.push(item) (no &)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_push_no_ref() {
    let code = r#"
pub fn add_items() {
    let mut vec = Vec::new()
    vec.push(1)
    vec.push(2)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(success, "Vec::push should compile. Error:\n{}", err);
    assert!(
        generated.contains("vec.push(1)") && generated.contains("vec.push(2)"),
        "Vec::push takes owned value, not &"
    );
}

/// Case 4: EventDataValue-style struct with HashMap - the 14-error dogfooding case
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_event_data_get_str_key() {
    let code = r#"
use std::collections::HashMap

pub enum EventDataValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
}

pub struct Event {
    data: HashMap<string, EventDataValue>,
}

impl Event {
    pub fn get_data_int(self, key: str) -> Option<i32> {
        match self.data.get(key) {
            Some(EventDataValue::Int(v)) => Some(*v),
            _ => None
        }
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // Must NOT have self.data.get(&key) - key: str is already &str
    assert!(
        !generated.contains("data.get(&key)"),
        "Borrow<&str> fix: key: str must not get extra &. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("data.get(key)"),
        "Should use key directly. Got:\n{}",
        generated
    );
    assert!(
        success,
        "Event.get_data_int must compile (fixes 14 HashMap errors). Error:\n{}",
        err
    );
}

/// Case 5: contains_key with str param - same fix
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_contains_key_str_param() {
    let code = r#"
use std::collections::HashMap

pub fn has_key(map: HashMap<string, i32>, key: str) -> bool {
    map.contains_key(key)
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    assert!(
        !generated.contains("contains_key(&key)"),
        "key: str must not get & for contains_key"
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Case 6: get with explicit &key in source - when key is str, strip to key
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_explicit_ref_str_param() {
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<string, i32>, key: str) -> Option<i32> {
    match map.get(&key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    // User wrote &key but key is str (→ &str). Must generate get(key) not get(&key)
    assert!(
        !generated.contains("map.get(&key)"),
        "Explicit &key with key: str must be stripped to avoid &&str"
    );
    assert!(success, "Must compile. Error:\n{}", err);
}
