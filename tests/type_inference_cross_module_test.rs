/// TDD Test: Cross-Module Type Inference
///
/// Pattern: Function calls across modules should propagate type information
/// Example: mod math { pub fn distance(a: f32, b: f32) -> f32 }
///          Using: let d = math::distance(self.x, 0.0)
///          Should infer: 0.0 as f32 based on function signature
///
/// This tests whether the compiler can look up function signatures
/// from other modules and use them for type inference.
use std::sync::atomic::{AtomicU64, Ordering};
use tempfile::TempDir;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn test_cross_module_function_call() {
    // Create a math module with a function
    let math_src = r#"
pub fn distance(a: f32, b: f32) -> f32 {
    let dx = a - b
    dx * dx
}
"#;

    // Create a main file that uses it
    let main_src = r#"
use math

struct Point {
    x: f32,
    y: f32,
}

impl Point {
    pub fn distance_from_origin(self) -> f32 {
        math::distance(self.x, 0.0)
    }
}
"#;

    let (test_dir, _counter) =
        setup_test_project(vec![("math.wj", math_src), ("main.wj", main_src)]);

    let output = compile_project(&test_dir, "main.wj");

    println!("\n=== Generated Rust ===\n{}\n", output);

    // The 0.0 should be inferred as f32 because math::distance expects f32
    assert!(
        output.contains("0.0_f32") || !output.contains("0.0_f64"),
        "0.0 should be f32, not f64: {}",
        output
    );
}

#[test]
fn test_cross_module_struct_field_access() {
    let types_src = r#"
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}
"#;

    let main_src = r#"
use types

pub fn calculate(v: Vector2) -> f32 {
    v.x + 0.5
}
"#;

    let (test_dir, _counter) =
        setup_test_project(vec![("types.wj", types_src), ("main.wj", main_src)]);

    let output = compile_project(&test_dir, "main.wj");

    println!("\n=== Generated Rust ===\n{}\n", output);

    // 0.5 should be f32 because v.x is f32
    assert!(
        output.contains("0.5_f32") || !output.contains("0.5_f64"),
        "0.5 should be f32: {}",
        output
    );
}

#[test]
fn test_cross_module_method_call() {
    let math_src = r#"
pub struct Calculator {
    pub factor: f32,
}

impl Calculator {
    pub fn multiply(self, value: f32) -> f32 {
        self.factor * value
    }
}
"#;

    let main_src = r#"
use math

pub fn calculate(calc: Calculator) -> f32 {
    calc.multiply(2.0)
}
"#;

    let (test_dir, _counter) =
        setup_test_project(vec![("math.wj", math_src), ("main.wj", main_src)]);

    let output = compile_project(&test_dir, "main.wj");

    println!("\n=== Generated Rust ===\n{}\n", output);

    // 2.0 should be f32 because Calculator::multiply expects f32
    assert!(
        output.contains("2.0_f32") || !output.contains("2.0_f64"),
        "2.0 should be f32: {}",
        output
    );
}

// Helper functions

fn setup_test_project(files: Vec<(&str, &str)>) -> (TempDir, u64) {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let test_dir = TempDir::new().expect("tempdir");

    for (filename, content) in files {
        let file_path = test_dir.path().join(filename);
        std::fs::write(&file_path, content).unwrap();
    }

    (test_dir, counter)
}

fn compile_project(test_dir: &TempDir, entry_file: &str) -> String {
    let source_file = test_dir.path().join(entry_file);

    windjammer::build_project(
        &source_file,
        test_dir.path(),
        windjammer::CompilationTarget::Rust,
        false,
    )
    .expect("Failed to compile Windjammer code");

    let rust_file = test_dir.path().join(entry_file.replace(".wj", ".rs"));
    std::fs::read_to_string(&rust_file).expect("Failed to read generated Rust file")
}
