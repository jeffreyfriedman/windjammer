//! Integration tests for multi-target code generation
//!
//! Tests that verify Windjammer can correctly compile to Rust, JavaScript, and WebAssembly
//! with consistent behavior across all targets.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper function to compile Windjammer code to a specific target
fn compile_to_target(source: &str, target: &str) -> Result<TempDir, String> {
    let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let source_file = temp_dir.path().join("test.wj");

    fs::write(&source_file, source).map_err(|e| format!("Failed to write source file: {}", e))?;

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
        .arg(temp_dir.path().join("build"))
        .output()
        .map_err(|e| format!("Failed to run wj: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(temp_dir)
}

#[test]
fn test_simple_function_rust() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "rust").expect("Rust compilation failed");

    // Verify generated files exist
    let build_dir = temp_dir.path().join("build");
    assert!(
        build_dir.join("Cargo.toml").exists(),
        "Cargo.toml should exist"
    );

    // Check for generated Rust files (typically test.rs for single file compilation)
    let rust_files: Vec<_> = fs::read_dir(&build_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
        .collect();

    assert!(!rust_files.is_empty(), "Should have at least one .rs file");
}

#[test]
fn test_simple_function_javascript() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    // Verify generated files exist
    let build_dir = temp_dir.path().join("build");
    assert!(
        build_dir.join("output.js").exists(),
        "output.js should exist"
    );
    assert!(
        build_dir.join("output.d.ts").exists(),
        "output.d.ts should exist"
    );
    assert!(
        build_dir.join("package.json").exists(),
        "package.json should exist"
    );

    // Verify JavaScript output is valid
    let js_content =
        fs::read_to_string(build_dir.join("output.js")).expect("Failed to read output.js");
    assert!(
        js_content.contains("export function add"),
        "Should have add function"
    );
    assert!(
        js_content.contains("export function main"),
        "Should have main function"
    );
    assert!(
        js_content.contains("Windjammer JavaScript transpiler"),
        "Should have header comment"
    );
}

#[test]
fn test_simple_function_wasm() {
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let result = add(2, 3)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "wasm").expect("WASM compilation failed");

    // Verify generated files exist
    let build_dir = temp_dir.path().join("build");
    assert!(
        build_dir.join("Cargo.toml").exists(),
        "Cargo.toml should exist"
    );
}

#[test]
fn test_typescript_definitions_quality() {
    let source = r#"
fn greet(name: string) -> string {
    "Hello, ${name}!"
}

fn calculate(x: int, y: int) -> int {
    x * y + 10
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let ts_content =
        fs::read_to_string(build_dir.join("output.d.ts")).expect("Failed to read output.d.ts");

    // Verify TypeScript definitions are generated correctly
    assert!(
        ts_content.contains("export declare function greet"),
        "Should declare greet function"
    );
    assert!(
        ts_content.contains("(name: string): string"),
        "Should have correct type signature"
    );
    assert!(
        ts_content.contains("export declare function calculate"),
        "Should declare calculate function"
    );
    assert!(
        ts_content.contains("(x: number, y: number): number"),
        "Should have correct numeric types"
    );
}

#[test]
fn test_javascript_async_detection() {
    let source = r#"
@async
fn fetch_data(url: string) -> string {
    "data"
}

@async
fn main() {
    let data = fetch_data("http://example.com").await
    println!("{}", data)
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let js_content =
        fs::read_to_string(build_dir.join("output.js")).expect("Failed to read output.js");

    // Verify async functions are detected
    assert!(
        js_content.contains("async function fetch_data"),
        "Should have async fetch_data"
    );
    assert!(
        js_content.contains("async function main"),
        "Should have async main"
    );
    assert!(js_content.contains("await"), "Should have await");
}

#[test]
fn test_javascript_struct_generation() {
    let source = r#"
struct Point {
    x: int,
    y: int,
}

fn main() {
    let p = Point { x: 10, y: 20 }
    println!("{}, {}", p.x, p.y)
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let js_content =
        fs::read_to_string(build_dir.join("output.js")).expect("Failed to read output.js");

    // Verify struct is generated as a class
    assert!(
        js_content.contains("export class Point"),
        "Should have Point class"
    );
    assert!(
        js_content.contains("constructor"),
        "Should have constructor"
    );
    assert!(js_content.contains("this.x"), "Should initialize x");
    assert!(js_content.contains("this.y"), "Should initialize y");
}

#[test]
fn test_javascript_enum_generation() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}

fn main() {
    let c = Color::Red
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let js_content =
        fs::read_to_string(build_dir.join("output.js")).expect("Failed to read output.js");

    // Verify enum is generated as frozen object with symbols
    assert!(
        js_content.contains("export const Color"),
        "Should have Color enum"
    );
    assert!(js_content.contains("Object.freeze"), "Should be frozen");
    assert!(js_content.contains("Symbol"), "Should use symbols");
}

#[test]
fn test_package_json_generation() {
    let source = r#"
fn hello() {
    println!("Hello, World!")
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let package_json =
        fs::read_to_string(build_dir.join("package.json")).expect("Failed to read package.json");

    // Verify package.json is valid JSON
    let json: serde_json::Value =
        serde_json::from_str(&package_json).expect("package.json should be valid JSON");

    assert!(json["name"].is_string(), "Should have name field");
    assert!(json["version"].is_string(), "Should have version field");
    assert!(json["type"] == "module", "Should be ES module");
    assert!(json["engines"].is_object(), "Should have engines field");
}

#[test]
fn test_javascript_control_flow() {
    let source = r#"
fn test_if(x: int) -> int {
    if x > 0 {
        1
    } else {
        -1
    }
}

fn test_loop() {
    let mut i = 0
    while i < 10 {
        i = i + 1
    }
}

fn test_for() {
    for i in 0..5 {
        println!("{}", i)
    }
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let js_content =
        fs::read_to_string(build_dir.join("output.js")).expect("Failed to read output.js");

    // Verify control flow structures are correctly generated
    assert!(js_content.contains("if ("), "Should have if statement");
    assert!(js_content.contains("} else {"), "Should have else block");
    assert!(js_content.contains("while ("), "Should have while loop");
    assert!(js_content.contains("for (const"), "Should have for loop");
}

#[test]
fn test_javascript_jsdoc_comments() {
    let source = r#"
fn multiply(a: int, b: int) -> int {
    a * b
}
"#;

    let temp_dir = compile_to_target(source, "javascript").expect("JavaScript compilation failed");

    let build_dir = temp_dir.path().join("build");
    let js_content =
        fs::read_to_string(build_dir.join("output.js")).expect("Failed to read output.js");

    // Verify JSDoc comments are generated
    assert!(
        js_content.contains("/**"),
        "Should have JSDoc comment start"
    );
    assert!(js_content.contains("@param"), "Should have @param tags");
    assert!(js_content.contains("@returns"), "Should have @returns tag");
    assert!(js_content.contains("*/"), "Should have JSDoc comment end");
}

#[test]
fn test_rust_output_still_works() {
    // Ensure Rust output hasn't regressed
    let source = r#"
fn fibonacci(n: int) -> int {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

fn main() {
    let result = fibonacci(10)
    println!("{}", result)
}
"#;

    let temp_dir = compile_to_target(source, "rust").expect("Rust compilation failed");

    let build_dir = temp_dir.path().join("build");
    let cargo_toml =
        fs::read_to_string(build_dir.join("Cargo.toml")).expect("Failed to read Cargo.toml");

    assert!(
        cargo_toml.contains("[package]"),
        "Should have package section"
    );
    assert!(cargo_toml.contains("name"), "Should have name field");
}

#[test]
fn test_all_targets_compile_same_source() {
    // Verify the same source can compile to all three targets without errors
    let source = r#"
fn add(a: int, b: int) -> int {
    a + b
}

fn main() {
    let x = add(2, 3)
    println!("{}", x)
}
"#;

    // Compile to all targets
    compile_to_target(source, "rust").expect("Rust compilation should succeed");
    compile_to_target(source, "javascript").expect("JavaScript compilation should succeed");
    compile_to_target(source, "wasm").expect("WASM compilation should succeed");
}
