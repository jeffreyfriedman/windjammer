// TDD Test: Integer literal inference in generic collections
//
// Bug: map.insert("k", 42) generates 42_i32 for HashMap<string, int> (int = i64)
// Expected: HashMap/BTreeMap value type and Vec element type should propagate to literals
//
// Tests: HashMap::insert, BTreeMap::insert, Vec::push, custom struct fields

use std::fs;
use std::process::Command;

fn compile_and_get_rust(wj_source: &str, test_name: &str) -> String {
    let output_dir = format!("/tmp/wj_int_inference_{}", test_name);
    fs::create_dir_all(&output_dir).unwrap();
    fs::write(format!("{}/test.wj", output_dir), wj_source).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            "--target",
            "rust",
            "--no-cargo",
            &format!("{}/test.wj", output_dir),
            "--output",
            &output_dir,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "Compilation should succeed, stderr: {}",
        stderr
    );

    fs::read_to_string(format!("{}/test.rs", output_dir))
        .expect("Generated Rust file not found")
}

#[test]
fn test_vec_push_i64_literal() {
    // Vec<int> = Vec<i64> → push(42) should generate 42_i64
    let wj_source = r#"
fn init_ids() -> Vec<int> {
    let mut ids: Vec<int> = Vec::new()
    ids.push(42)
    ids.push(100)
    ids
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "vec_push_i64");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("42_i64") && rust_code.contains("100_i64"),
        "Vec<int>.push() should infer i64 literals, got:\n{}",
        rust_code
    );
}

#[test]
fn test_hashmap_insert_i64_literal() {
    // HashMap<string, int> → insert("k", 42) should generate 42_i64
    let wj_source = r#"
use std::collections::HashMap

fn init_map() -> HashMap<string, int> {
    let mut map: HashMap<string, int> = HashMap::new()
    map.insert("a", 42)
    map.insert("b", 100)
    map
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "hashmap_insert_i64");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("42_i64") && rust_code.contains("100_i64"),
        "HashMap<string, int>.insert() should infer i64 value literals, got:\n{}",
        rust_code
    );
}

#[test]
fn test_btreemap_insert_i64_literal() {
    // BTreeMap<string, int> → insert("k", 42) should generate 42_i64
    let wj_source = r#"
use std::collections::BTreeMap

fn init_map() -> BTreeMap<string, int> {
    let mut map: BTreeMap<string, int> = BTreeMap::new()
    map.insert("a", 42)
    map.insert("b", 100)
    map
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "btreemap_insert_i64");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("42_i64") && rust_code.contains("100_i64"),
        "BTreeMap<string, int>.insert() should infer i64 value literals, got:\n{}",
        rust_code
    );
}

#[test]
fn test_struct_field_option_vec_int() {
    // Option<Vec<int>> field → Some(vec![2, 3]) should generate 2_i64, 3_i64
    let wj_source = r#"
struct Node {
    pub value: int,
    pub children: Option<Vec<int>>
}

fn main() {
    let n = Node { value: 1, children: Some(vec![2, 3]) }
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "struct_option_vec");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("2_i64") && rust_code.contains("3_i64"),
        "Option<Vec<int>> struct field should infer i64 for vec! elements, got:\n{}",
        rust_code
    );
}

#[test]
fn test_struct_field_vec_option_int() {
    // Vec<Option<int>> field → [Some(1), Some(2)] should generate 1_i64, 2_i64
    let wj_source = r#"
struct Data {
    pub values: Vec<Option<int>>
}

fn main() {
    let d = Data { values: vec![Some(1), Some(2)] }
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "struct_vec_option");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("1_i64") && rust_code.contains("2_i64"),
        "Vec<Option<int>> struct field should infer i64 for Some() args, got:\n{}",
        rust_code
    );
}

#[test]
fn test_struct_field_option_option_int() {
    // Option<Option<int>> field → Some(Some(42)) should generate 42_i64
    let wj_source = r#"
struct Wrapper {
    pub inner: Option<Option<int>>
}

fn main() {
    let w = Wrapper { inner: Some(Some(42)) }
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "struct_option_option");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("42_i64"),
        "Option<Option<int>> struct field should infer i64 for inner Some() arg, got:\n{}",
        rust_code
    );
}

#[test]
fn test_custom_struct_field_hashmap_insert() {
    // Struct with HashMap field → self.field.insert() should infer from field type
    let wj_source = r#"
use std::collections::HashMap

struct DataStore {
    counts: HashMap<string, int>
}

impl DataStore {
    pub fn add(self, key: string, value: int) {
        self.counts.insert(key, value)
    }
}

fn main() {
    let mut store = DataStore { counts: HashMap::new() }
    store.counts.insert("x", 42)
    store.counts.insert("y", 99)
}
"#;

    let rust_code = compile_and_get_rust(wj_source, "struct_field_hashmap");
    eprintln!("Generated Rust:\n{}", rust_code);

    assert!(
        rust_code.contains("42_i64") && rust_code.contains("99_i64"),
        "Struct field HashMap<string, int>.insert() should infer i64, got:\n{}",
        rust_code
    );
}
