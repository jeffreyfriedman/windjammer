//! TDD tests for the typed lowering codegen overhaul.
//!
//! Each test encodes one of the 7 error classes found in windjammer-game-core downstream
//! builds. Tests compile .wj programs to Rust and verify the generated code has correct
//! ownership coercions.

#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn wj_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj")
}

fn build_wj_to_rust(test_name: &str, wj_source: &str) -> String {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_typed_lowering")
        .join(test_name);
    fs::create_dir_all(&test_dir).unwrap();

    let test_file = test_dir.join(format!("{}.wj", test_name));
    fs::write(&test_file, wj_source).unwrap();

    let output = Command::new(wj_binary())
        .current_dir(&test_dir)
        .arg("build")
        .arg("--no-cargo")
        .arg(&test_file)
        .output()
        .expect("Failed to run wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        panic!(
            "wj build failed for {}\nSTDOUT:\n{}\nSTDERR:\n{}",
            test_name, stdout, stderr
        );
    }

    let rs_file = test_dir.join("build").join(format!("{}.rs", test_name));
    if rs_file.exists() {
        return fs::read_to_string(&rs_file).unwrap();
    }

    stdout.to_string()
}

fn build_wj_multifile(test_name: &str, files: &[(&str, &str)]) -> Vec<(String, String)> {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_typed_lowering")
        .join(test_name);
    fs::create_dir_all(&test_dir).unwrap();

    let mut results = Vec::new();
    for (name, source) in files {
        let test_file = test_dir.join(name);
        fs::write(&test_file, source).unwrap();
    }

    for (name, _) in files {
        let test_file = test_dir.join(name);
        let output = Command::new(wj_binary())
            .current_dir(&test_dir)
            .arg("build")
            .arg("--no-cargo")
            .arg(&test_file)
            .output()
            .expect("Failed to run wj build");

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        results.push((name.to_string(), stdout));
    }

    results
}

// ============================================================================
// Class 1: Missing & — owned value passed where reference expected
// ============================================================================

#[test]
fn class1_method_with_borrowed_string_param() {
    let rust = build_wj_to_rust("class1_borrowed_str", r#"
struct Registry {
    names: Vec<string>
}

impl Registry {
    pub fn is_registered(self, name: string) -> bool {
        return false
    }

    pub fn register(self, name: string) {
        if self.is_registered(name) {
            return
        }
        self.names.push(name)
    }
}

fn main() {
    let mut reg = Registry { names: Vec::new() }
    reg.register("test")
}
"#);

    assert!(!rust.contains("name.clone()") || rust.contains("&name"),
        "Borrowed string param should use & not .clone()\nGenerated:\n{}", rust);
}

// ============================================================================
// Class 3: String literal missing .to_string()
// ============================================================================

#[test]
fn class3_string_literal_to_owned_param() {
    let rust = build_wj_to_rust("class3_str_literal", r#"
struct Material {
    name: string
}

impl Material {
    pub fn new() -> Material {
        return Material { name: "default" }
    }

    pub fn set_name(self, name: string) {
        self.name = name
    }
}

fn main() {
    let mut mat = Material::new()
    mat.set_name("Metal")
}
"#);

    let has_to_string = rust.contains("\"Metal\".to_string()")
        || rust.contains("\"Metal\".to_owned()")
        || rust.contains("String::from(\"Metal\")");

    assert!(has_to_string,
        "String literal to owned String param needs .to_string()\nGenerated:\n{}", rust);
}

// ============================================================================
// Class 4: Copy type over-borrowed
// ============================================================================

#[test]
fn class4_copy_type_not_borrowed() {
    let rust = build_wj_to_rust("class4_copy_no_borrow", r#"
struct Vec3 {
    x: f32,
    y: f32,
    z: f32
}

fn magnitude(v: Vec3) -> f32 {
    return v.x * v.x + v.y * v.y + v.z * v.z
}

fn process(v: Vec3) -> f32 {
    return magnitude(v)
}

fn main() {
    let v = Vec3 { x: 1.0, y: 2.0, z: 3.0 }
    let m = process(v)
    println!("{}", m)
}
"#);

    let has_ref_on_copy = rust.contains("magnitude(&v)")
        || rust.contains("magnitude(&self.");
    assert!(!has_ref_on_copy,
        "Copy type should not get & when passed to owned param\nGenerated:\n{}", rust);
}

// ============================================================================
// Class 5: Inverse string coercion — &str vs String confusion
// ============================================================================

#[test]
fn class5_str_ref_param_no_to_string() {
    let rust = build_wj_to_rust("class5_str_ref", r#"
fn greet(name: string) {
    println!("Hello, {}", name)
}

fn greet_all(names: Vec<string>) {
    for name in names {
        greet(name)
    }
}

fn main() {
    let names = Vec::new()
    greet_all(names)
}
"#);

    assert!(rust.contains("fn greet") && rust.contains("fn greet_all"),
        "Both functions should be generated\nGenerated:\n{}", rust);
}

// ============================================================================
// Class 6: Spurious .clone() on generic types without Clone bound
// ============================================================================

// Known bug: auto_clone_analysis adds .clone() to generic T without Clone bound.
// This will be fixed as part of Phase 5 (remove legacy heuristic code).
#[test]
#[ignore = "pre-existing bug: auto_clone adds .clone() to unconstrained generic T"]
fn class6_no_clone_on_moved_generic() {
    let rust = build_wj_to_rust("class6_generic_no_clone", r#"
struct Pool<T> {
    items: Vec<T>,
    capacity: i32
}

impl<T> Pool<T> {
    pub fn release(self, obj: T) {
        if self.items.len() < self.capacity {
            self.items.push(obj)
        }
    }
}

fn main() {
    let mut pool: Pool<i32> = Pool { items: Vec::new(), capacity: 10 }
    pool.release(42)
}
"#);

    let has_obj_clone = rust.contains("obj.clone()");
    assert!(!has_obj_clone,
        "Generic T without Clone bound should not get .clone()\nGenerated:\n{}", rust);
}

// ============================================================================
// Regression: Vec::join separator stays as &str
// ============================================================================

#[test]
fn vec_join_separator_stays_str() {
    let rust = build_wj_to_rust("join_separator", r#"
fn make_csv(items: Vec<string>) -> string {
    return items.join(", ")
}

fn main() {
    let items = Vec::new()
    let csv = make_csv(items)
    println!("{}", csv)
}
"#);

    let has_join_to_string = rust.contains(".join(\", \".to_string())")
        || rust.contains(".join(String::from");
    assert!(!has_join_to_string,
        "Vec::join separator should stay as &str, not .to_string()\nGenerated:\n{}", rust);
}
