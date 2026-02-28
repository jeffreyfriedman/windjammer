/// TDD Test: HashMap/BTreeMap with String keys - incorrect &String argument
/// 
/// Problem: HashMap<String, T>.contains_key(&name) where name is &String
/// Error: E0277: the trait bound `String: Borrow<&String>` is not satisfied
/// 
/// Example:
/// ```windjammer
/// let map = HashMap::new()
/// map.insert("key", "value")
/// let key_name = "key"
/// if map.contains_key(&key_name) { ... }  // ERROR: &String not compatible with HashMap<String, T>
/// ```
/// 
/// Solution: Convert &String to &str when used with HashMap/BTreeMap methods
/// map.contains_key(&key_name) -> map.contains_key(key_name) or map.contains_key(&**key_name)

use std::fs;
use std::process::Command;

#[test]
fn test_hashmap_string_key_contains() {
    let source = r#"
use std::collections::HashMap

fn check_exists(map: HashMap<string, int>, key: string) -> bool {
    map.contains_key(&key)
}

fn main() {
    let map = HashMap::new()
    map.insert("foo", 42)
    let result = check_exists(map, "foo")
    assert_eq(result, true)
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();
    
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();
    
    let out_dir = test_dir.join("out");
    
    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");
    
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");
    
    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    // Verify HashMap method calls with String keys work correctly
    // Should NOT generate &key which causes E0277: String: Borrow<&String>
    // Should generate key or &**key or key.as_str()
    assert!(
        !generated.contains("contains_key(&key)") ||
        generated.contains("contains_key(key)") ||
        generated.contains("contains_key(&**key)") ||
        generated.contains("contains_key(key.as_str())"),
        "HashMap::contains_key should not use &String directly"
    );
    
    fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn test_hashmap_string_key_get() {
    let source = r#"
use std::collections::HashMap

fn get_value(map: HashMap<string, int>, key: string) -> Option<int> {
    map.get(&key).cloned()
}

fn main() {
    let map = HashMap::new()
    map.insert("foo", 42)
    let result = get_value(map, "foo")
    assert_eq(result, Some(42))
}
"#;

    let temp_dir = std::env::temp_dir();
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();
    
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();
    
    let out_dir = test_dir.join("out");
    
    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");
    
    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");
    
    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    fs::remove_dir_all(&test_dir).ok();
}
