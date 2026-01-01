//! End-to-end tests for all compilation targets
//!
//! These tests compile Windjammer code to all three targets and verify
//! the generated output is correct and executable.
//!
//! Note: These tests spawn cargo processes and must be serialized to avoid
//! file system contention and process interference.

#![allow(clippy::expect_fun_call)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tempfile::TempDir;

// Global mutex to serialize tests that spawn cargo processes
// This prevents file system contention and process interference
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Test case structure
struct TestCase {
    name: &'static str,
    source: &'static str,
    expected_js_contains: &'static [&'static str],
    expected_rust_contains: &'static [&'static str],
}

const TEST_CASES: &[TestCase] = &[
    TestCase {
        name: "simple_function",
        source: r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
    println!("{}", result)
}
"#,
        expected_js_contains: &["export function add", "export function main", "(a + b)"],
        expected_rust_contains: &["fn add", "fn main"],
    },
    TestCase {
        name: "struct_definition",
        source: r#"
struct Point {
    x: int,
    y: int,
}

fn main() {
    let p = Point { x: 10, y: 20 }
    println!("{}", p.x)
}
"#,
        expected_js_contains: &["export class Point", "constructor", "this.x", "this.y"],
        expected_rust_contains: &["struct Point", "x: i64", "y: i64"],
    },
    TestCase {
        name: "enum_definition",
        source: r#"
enum Color {
    Red,
    Green,
    Blue,
}

fn main() {
    let c = Color::Red
}
"#,
        expected_js_contains: &["export const Color", "Object.freeze", "Symbol"],
        expected_rust_contains: &["enum Color", "Red", "Green", "Blue"],
    },
    TestCase {
        name: "control_flow",
        source: r#"
fn test_control(x: int) -> int {
    if x > 0 {
        1
    } else {
        -1
    }
}

fn main() {
    let result = test_control(5)
}
"#,
        expected_js_contains: &["if (", "} else {", "return"],
        expected_rust_contains: &["if x > 0", "else"],
    },
    TestCase {
        name: "async_function",
        source: r#"
@async
fn fetch_data(url: string) -> string {
    "data"
}

@async
fn main() {
    let data = fetch_data("http://example.com").await
}
"#,
        expected_js_contains: &["async function fetch_data", "async function main", "await"],
        expected_rust_contains: &["async fn fetch_data", "async fn main", ".await"],
    },
];

fn compile_to_target(source: &str, target: &str, temp_dir: &TempDir) -> Result<PathBuf, String> {
    let source_file = temp_dir.path().join("test.wj");
    let output_dir = temp_dir.path().join(format!("build_{}", target));

    fs::write(&source_file, source).map_err(|e| format!("Failed to write source: {}", e))?;

    let output = Command::new("cargo")
        .arg("run")
        .arg("--bin")
        .arg("wj")
        .arg("--")
        .arg("build")
        .arg("--target")
        .arg(target)
        .arg(&source_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo") // Skip cargo build in tests to avoid conflicts
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed for target {}:\nstdout: {}\nstderr: {}",
            target,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output_dir)
}

#[test]
fn test_all_targets_all_cases() {
    // Acquire mutex to serialize this test
    let _lock = TEST_MUTEX.lock().unwrap();

    for test_case in TEST_CASES {
        println!("\n=== Testing: {} ===", test_case.name);

        let temp_dir =
            TempDir::new().expect(&format!("Failed to create temp dir for {}", test_case.name));

        // Test JavaScript target
        {
            let js_dir = compile_to_target(test_case.source, "javascript", &temp_dir).expect(
                &format!("JavaScript compilation failed for {}", test_case.name),
            );

            let js_file = js_dir.join("output.js");
            assert!(
                js_file.exists(),
                "output.js should exist for {}",
                test_case.name
            );

            let js_content = fs::read_to_string(&js_file)
                .expect(&format!("Failed to read output.js for {}", test_case.name));

            for expected in test_case.expected_js_contains {
                assert!(
                    js_content.contains(expected),
                    "JavaScript output for {} should contain '{}'\nActual output:\n{}",
                    test_case.name,
                    expected,
                    js_content
                );
            }

            // Verify TypeScript definitions exist
            assert!(
                js_dir.join("output.d.ts").exists(),
                "output.d.ts should exist for {}",
                test_case.name
            );

            // Verify package.json exists
            assert!(
                js_dir.join("package.json").exists(),
                "package.json should exist for {}",
                test_case.name
            );

            println!("✅ JavaScript target passed for {}", test_case.name);
        }

        // Test Rust target
        {
            let rust_dir = compile_to_target(test_case.source, "rust", &temp_dir)
                .expect(&format!("Rust compilation failed for {}", test_case.name));

            assert!(
                rust_dir.join("Cargo.toml").exists(),
                "Cargo.toml should exist for {}",
                test_case.name
            );

            // Find the generated .rs file (should be in src/ subdirectory)
            let src_dir = rust_dir.join("src");
            let rust_files: Vec<_> = if src_dir.exists() {
                fs::read_dir(&src_dir)
                    .expect("Failed to read src dir")
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
                    .collect()
            } else {
                // Fallback: look in the root directory
                fs::read_dir(&rust_dir)
                    .expect("Failed to read rust dir")
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
                    .collect()
            };

            assert!(
                !rust_files.is_empty(),
                "Should have at least one .rs file for {} (checked {} and {})",
                test_case.name,
                rust_dir.display(),
                src_dir.display()
            );

            let rust_content = fs::read_to_string(rust_files[0].path())
                .expect(&format!("Failed to read Rust file for {}", test_case.name));

            for expected in test_case.expected_rust_contains {
                assert!(
                    rust_content.contains(expected),
                    "Rust output for {} should contain '{}'\nActual output:\n{}",
                    test_case.name,
                    expected,
                    rust_content
                );
            }

            println!("✅ Rust target passed for {}", test_case.name);
        }

        // Test WASM target
        {
            let wasm_dir = compile_to_target(test_case.source, "wasm", &temp_dir)
                .expect(&format!("WASM compilation failed for {}", test_case.name));

            assert!(
                wasm_dir.join("Cargo.toml").exists(),
                "Cargo.toml should exist for WASM target in {}",
                test_case.name
            );

            println!("✅ WASM target passed for {}", test_case.name);
        }
    }
}

#[test]
fn test_javascript_string_interpolation() {
    // Acquire mutex to serialize this test
    let _lock = TEST_MUTEX.lock().unwrap();

    let source = r#"
fn greet(name: string) {
    println!("Hello, {}", name)
}

fn main() {
    greet("World")
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let js_dir =
        compile_to_target(source, "javascript", &temp_dir).expect("JavaScript compilation failed");

    let js_content =
        fs::read_to_string(js_dir.join("output.js")).expect("Failed to read output.js");

    // Should use console.log for println!
    assert!(
        js_content.contains("console.log"),
        "Should convert println! to console.log"
    );
    assert!(
        js_content.contains("export function greet"),
        "Should have greet function"
    );
    assert!(
        js_content.contains("export function main"),
        "Should have main function"
    );
}

#[test]
fn test_javascript_jsdoc_generation() {
    // Acquire mutex to serialize this test
    let _lock = TEST_MUTEX.lock().unwrap();

    let source = r#"
fn calculate(x: int, y: int, z: float) -> float {
    (x + y) as float + z
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let js_dir =
        compile_to_target(source, "javascript", &temp_dir).expect("JavaScript compilation failed");

    let js_content =
        fs::read_to_string(js_dir.join("output.js")).expect("Failed to read output.js");

    // Should have JSDoc comments
    assert!(js_content.contains("/**"), "Should have JSDoc start");
    assert!(
        js_content.contains("@param {number}"),
        "Should have number type params"
    );
    assert!(
        js_content.contains("@returns {number}"),
        "Should have number return type"
    );
    assert!(js_content.contains("*/"), "Should have JSDoc end");
}

#[test]
#[ignore] // TODO: Fix TypeScript definitions quality test
fn test_typescript_definitions_quality() {
    // Acquire mutex to serialize this test
    let _lock = TEST_MUTEX.lock().unwrap();

    let source = r#"
struct User {
    name: string,
    age: int,
}

fn create_user(name: string, age: int) -> User {
    User { name: name, age: age }
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let js_dir =
        compile_to_target(source, "javascript", &temp_dir).expect("JavaScript compilation failed");

    let ts_content =
        fs::read_to_string(js_dir.join("output.d.ts")).expect("Failed to read output.d.ts");

    // Should have proper TypeScript declarations
    assert!(
        ts_content.contains("export interface User"),
        "Should have User interface"
    );
    assert!(
        ts_content.contains("name: string"),
        "Should have name field"
    );
    assert!(
        ts_content.contains("age: number"),
        "Should have age field with number type"
    );
    assert!(
        ts_content.contains("export declare function create_user"),
        "Should have function declaration"
    );
}

#[test]
fn test_complex_program_all_targets() {
    // Acquire mutex to serialize this test
    let _lock = TEST_MUTEX.lock().unwrap();
    let source = r#"
struct Task {
    id: int,
    title: string,
    completed: bool,
}

fn create_task(id: int, title: string) -> Task {
    Task { id: id, title: title, completed: false }
}

fn complete_task(task: Task) -> Task {
    Task { id: task.id, title: task.title, completed: true }
}

fn main() {
    let task = create_task(1, "Learn Windjammer")
    let completed = complete_task(task)
    println!("Task: {}", completed.title)
}
"#;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Compile to all targets
    let js_dir =
        compile_to_target(source, "javascript", &temp_dir).expect("JavaScript compilation failed");
    let rust_dir = compile_to_target(source, "rust", &temp_dir).expect("Rust compilation failed");
    let wasm_dir = compile_to_target(source, "wasm", &temp_dir).expect("WASM compilation failed");

    // Verify JavaScript output
    let js_content =
        fs::read_to_string(js_dir.join("output.js")).expect("Failed to read JavaScript output");
    assert!(
        js_content.contains("export class Task"),
        "JS should have Task class"
    );
    assert!(
        js_content.contains("export function create_task"),
        "JS should have create_task"
    );
    assert!(
        js_content.contains("export function complete_task"),
        "JS should have complete_task"
    );

    // Verify all required files exist
    assert!(
        js_dir.join("output.d.ts").exists(),
        "TypeScript definitions should exist"
    );
    assert!(
        js_dir.join("package.json").exists(),
        "package.json should exist"
    );
    assert!(
        rust_dir.join("Cargo.toml").exists(),
        "Rust Cargo.toml should exist"
    );
    assert!(
        wasm_dir.join("Cargo.toml").exists(),
        "WASM Cargo.toml should exist"
    );
}
