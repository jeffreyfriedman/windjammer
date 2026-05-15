#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Tests for Phase 2 String Parameter Optimization
///
/// These tests verify the smart &str optimization:
/// - Functions without Vec<String> methods → &str (performance)
/// - Functions with Vec<String> methods → &String (correctness)
/// - Passthrough analysis (if passed to &String function → use &String)
/// - Mixed parameters (per-parameter granularity)
#[path = "../common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_function_without_vec_methods_uses_str_ref() {
    // TDD: Function that only reads string param should use &str
    let code = r#"
pub fn log(msg: string) {
    println!("{}", msg)
}

fn main() {
    log("Hello, World!")
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 2 OPTIMIZATION: Should generate &str (no Vec<String> methods)
    assert!(
        generated.contains("fn log(msg: &str)"),
        "Expected &str parameter (no Vec<String> methods). Generated:\n{}",
        generated
    );

    // String literal should pass directly (no allocation)
    assert!(
        generated.contains(r#"log("Hello, World!")"#),
        "Expected direct string literal (no conversion). Generated:\n{}",
        generated
    );
}

#[test]
fn test_function_with_vec_contains_uses_string_ref() {
    // TDD: Function calling Vec<String>::contains must use &String
    let code = r#"
pub fn has_item(items: Vec<string>, id: string) -> bool {
    items.contains(&id)
}

fn main() {
    let items = vec!["foo".to_string(), "bar".to_string()]
    let result = has_item(items, "foo")
    println!("{}", result)
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 1 BASELINE: Should generate &String (Vec<String>::contains needs it)
    assert!(
        generated.contains("fn has_item(items: &Vec<String>, id: &String)"),
        "Expected &String parameter (Vec<String>::contains). Generated:\n{}",
        generated
    );

    // String literal must be converted to &String
    assert!(
        generated.contains(r#"has_item(&items, &"foo".to_string())"#),
        "Expected converted string literal. Generated:\n{}",
        generated
    );
}

#[test]
fn test_passthrough_to_string_ref_function() {
    // TDD: Wrapper function passing to &String function should also use &String
    let code = r#"
struct Inventory {
    items: Vec<string>
}

impl Inventory {
    fn has(self, id: string) -> bool {
        self.items.contains(&id)
    }
}

fn check_item(inv: Inventory, item_id: string) -> bool {
    inv.has(item_id)
}

fn main() {
    let inv = Inventory { items: vec!["sword".to_string()] }
    let result = check_item(inv, "sword")
    println!("{}", result)
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 2 PASSTHROUGH: check_item should use &String (calls has which needs &String).
    // inv may be `Inventory` or `&Inventory` depending on self/borrow inference.
    assert!(
        generated.contains("item_id: &String")
            && (generated.contains("fn check_item(inv: &Inventory, item_id: &String)")
                || generated.contains("fn check_item(inv: Inventory, item_id: &String)")),
        "Expected &String on item_id and passthrough to has (need &String). Generated:\n{}",
        generated
    );

    // Inventory::has should use &String (calls Vec<String>::contains)
    assert!(
        generated.contains("fn has(&self, id: &String)"),
        "Expected &String parameter (Vec<String>::contains). Generated:\n{}",
        generated
    );
}

#[test]
fn test_mixed_parameters_granular_optimization() {
    // TDD: Different parameters in same function can have different optimizations
    let code = r#"
pub fn process(items: Vec<string>, search: string, msg: string) -> bool {
    if items.contains(&search) {
        println!("{}", msg)
        true
    } else {
        false
    }
}

fn main() {
    let items = vec!["apple".to_string(), "banana".to_string()]
    let result = process(items, "apple", "Found it!")
    println!("{}", result)
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 2 MIXED OPTIMIZATION:
    // - search: &String (passed to Vec<String>::contains)
    // - msg: &str (only passed to println, no Vec methods)
    assert!(
        generated.contains("fn process(items: &Vec<String>, search: &String, msg: &str)"),
        "Expected mixed optimization: search=&String, msg=&str. Generated:\n{}",
        generated
    );
}

#[test]
fn test_phase1_baseline_still_works() {
    // TDD: Phase 2 optimization - function using format!() can use &str
    let code = r#"
pub fn greet(name: string) -> string {
    format!("Hello, {}!", name)
}

fn main() {
    let result = greet("World")
    println!("{}", result)
}
"#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    println!("Generated:\n{}", generated);

    // PHASE 2 OPTIMIZATION: format!() works with &str, so optimize to &str
    assert!(
        generated.contains("fn greet(name: &str)"),
        "Expected &str parameter (Phase 2 optimization). Generated:\n{}",
        generated
    );

    // String literal passed directly (no conversion for &str)
    assert!(
        generated.contains(r#"greet("World")"#),
        "Expected direct string literal (no conversion for &str). Generated:\n{}",
        generated
    );

    // Verify it compiles with rustc
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let rs_file = temp_dir.path().join("test.rs");
    fs::write(&rs_file, &generated).expect("Failed to write Rust file");

    let rustc_output = Command::new("rustc")
        .args([
            "--crate-type",
            "bin",
            "--edition",
            "2021",
            rs_file.to_str().unwrap(),
            "--out-dir",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Rustc compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }
}
