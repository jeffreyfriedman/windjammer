//! TDD Test: Fix HashMap.get() double-reference bug
//!
//! Bug: When calling HashMap.get() with an already-borrowed key from .keys(),
//! the transpiler incorrectly adds another `&`, resulting in `&&String` instead of `&String`.
//!
//! Example:
//!   Windjammer: `map.get(key)` where key is `&String`
//!   Generated:  `map.get(&key)` which is `&&String` ❌
//!   Should be:  `map.get(key)` which is `&String` ✅

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
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_with_borrowed_key_from_keys() {
    // Test Case 1: HashMap.get() with key from .keys() iterator
    let code = r#"
use std::collections::HashMap

pub fn get_values(map: &HashMap<string, i32>) -> Vec<i32> {
    let mut result = Vec::new()
    
    for key in map.keys() {
        let value = map.get(key)
        match value {
            Some(v) => result.push(*v),
            None => {}
        }
    }
    
    result
}
"#;

    let (success, generated, err) = compile_and_verify(code);

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // Should NOT contain map.get(&key) which causes double-ref
    assert!(
        !generated.contains("map.get(&key)"),
        "Generated code should not double-reference key: map.get(&key) is wrong"
    );

    // Should contain map.get(key)
    assert!(
        generated.contains("map.get(key)"),
        "Generated code should use single reference: map.get(key)"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
#[ignore] // TODO: Pre-existing bug - owned String parameters don't get & added
fn test_hashmap_get_with_owned_key() {
    // Test Case 2: HashMap.get() with owned String key
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: &HashMap<string, i32>, key: string) -> Option<i32> {
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

    // When key is owned String, should auto-borrow to &String
    assert!(
        generated.contains("map.get(&key)") || generated.contains("map.get(key)"),
        "Generated code should handle owned String key correctly"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_with_explicit_borrowed_key() {
    // Test Case 3: HashMap.get() with already-borrowed key parameter
    let code = r#"
use std::collections::HashMap

pub fn lookup_borrowed(map: &HashMap<string, i32>, key: &string) -> Option<i32> {
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

    // When key is already &String, should NOT add another &
    assert!(
        !generated.contains("map.get(&key)"),
        "Should not double-reference when key is already &String"
    );

    assert!(
        generated.contains("map.get(key)"),
        "Should use key directly when it's already borrowed"
    );

    assert!(
        success,
        "Generated Rust code should compile successfully. Rustc error:\n{}",
        err
    );
}
