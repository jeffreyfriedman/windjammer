/// Test: Struct pattern matching should use shorthand field patterns
///
/// Bug: The compiler generates `Shape::Triangle { base: base, height: height }`
/// instead of the idiomatic `Shape::Triangle { base, height }` when the
/// binding name matches the field name.
///
/// Root cause: Codegen always emits `field: binding` even when field == binding.
///
/// Fix: When field name equals binding name, use shorthand syntax.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn compile_wj_source(test_name: &str, source: &str) -> String {
    let wj_binary = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join("wj");

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("test_{}", test_name));

    fs::create_dir_all(&test_dir).unwrap();

    let test_file = test_dir.join(format!("{}.wj", test_name));
    fs::write(&test_file, source).unwrap();

    let output = Command::new(&wj_binary)
        .current_dir(&test_dir)
        .arg("build")
        .arg(&test_file)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj build");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDOUT:\n{}", stdout);
    println!("STDERR:\n{}", stderr);

    let rust_file = test_dir.join("build").join(format!("{}.rs", test_name));
    fs::read_to_string(&rust_file)
        .unwrap_or_else(|e| panic!("Failed to read generated Rust file {:?}: {}", rust_file, e))
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_pattern_uses_shorthand_when_names_match() {
    let source = r#"
enum Shape {
    Circle(f32),
    Triangle { base: f32, height: f32 },
}

impl Shape {
    fn area(self) -> f32 {
        match self {
            Shape::Circle(r) => 3.14 * r * r,
            Shape::Triangle { base, height } => 0.5 * base * height,
        }
    }
}

fn main() {
    let s = Shape::Triangle { base: 3.0, height: 4.0 }
    println!("{}", s.area())
}
"#;

    let rust_code = compile_wj_source("struct_pattern_shorthand", source);
    println!("Generated Rust:\n{}", rust_code);

    // Should use shorthand: `Shape::Triangle { base, height }`
    // NOT verbose: `Shape::Triangle { base: base, height: height }`
    assert!(
        !rust_code.contains("base: base"),
        "Should use shorthand field pattern 'base' not 'base: base'\nGenerated:\n{}",
        rust_code
    );
    assert!(
        !rust_code.contains("height: height"),
        "Should use shorthand field pattern 'height' not 'height: height'\nGenerated:\n{}",
        rust_code
    );

    // Verify it still has the correct pattern (shorthand form)
    assert!(
        rust_code.contains("Shape::Triangle { base, height }"),
        "Should generate shorthand struct pattern\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_pattern_keeps_verbose_when_names_differ() {
    // When binding name differs from field name, keep verbose syntax
    let source = r#"
enum Shape {
    Rectangle { width: f32, height: f32 },
}

impl Shape {
    fn describe(self) -> string {
        match self {
            Shape::Rectangle { width: w, height: h } => format!("{}x{}", w, h),
        }
    }
}

fn main() {
    let s = Shape::Rectangle { width: 3.0, height: 4.0 }
    println!("{}", s.describe())
}
"#;

    let rust_code = compile_wj_source("struct_pattern_verbose", source);
    println!("Generated Rust:\n{}", rust_code);

    // When names differ, should keep verbose: `width: w, height: h`
    assert!(
        rust_code.contains("width: w") || rust_code.contains("width: _w"),
        "Should keep verbose pattern when binding differs from field\nGenerated:\n{}",
        rust_code
    );
}
