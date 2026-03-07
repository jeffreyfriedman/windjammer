/// TDD Test: HashMap/BTreeMap with String keys - E0277 fix
///
/// Problem: HashMap<String, T>.contains_key(&name) where name is &String
/// creates &&String which doesn't satisfy Borrow trait
///
/// Solution: Strip explicit & for borrowed String params passed to HashMap key methods
use std::fs;
use std::process::Command;

fn run_wj_test(source: &str) -> String {
    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    if !output.status.success() {
        panic!(
            "wj build failed:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    fs::remove_dir_all(&test_dir).ok();
    generated
}

#[test]
fn test_hashmap_string_key_contains() {
    let source = r#"
use std::collections::HashMap

fn check_exists(map: HashMap<string, int>, key: string) -> bool {
    map.contains_key(&key)
}

fn main() {
    let mut map = HashMap::new()
    map.insert("foo".to_string(), 42)
    let result = check_exists(map, "foo")
    println("{}", result)
}
"#;

    let generated = run_wj_test(source);
    println!("Generated code:\n{}", generated);

    // WINDJAMMER DESIGN: String params infer to &str (not &String!)
    // - Read-only string param → &str (idiomatic Rust)
    // - User wrote `&key` → we generate `contains_key(key)` (already &str, strip redundant &)
    assert!(
        generated.contains("key: &str"),
        "Should generate &str parameter. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("contains_key(key)"),
        "Should pass key directly (already &str, strip &). Generated:\n{}",
        generated
    );
}

#[test]
fn test_hashmap_string_key_get() {
    let source = r#"
use std::collections::HashMap

fn get_value(map: HashMap<string, int>, key: string) -> Option<int> {
    map.get(&key).cloned()
}

fn main() {
    let mut map = HashMap::new()
    map.insert("foo".to_string(), 42)
    let result = get_value(map, "foo")
    println("{}", result)
}
"#;

    let generated = run_wj_test(source);
    println!("Generated code:\n{}", generated);

    // WINDJAMMER DESIGN: String params infer to &str (not &String!)
    // - Read-only string param → &str (idiomatic Rust)
    // - User wrote `&key` → we generate `map.get(key)` (already &str, strip redundant &)
    assert!(
        generated.contains("key: &str"),
        "Should generate &str parameter. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("map.get(key)"),
        "Should pass key directly (already &str, strip &). Generated:\n{}",
        generated
    );
}
