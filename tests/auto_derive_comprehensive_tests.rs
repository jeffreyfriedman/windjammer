//! Comprehensive @auto/@derive Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust derive attributes, including:
//! - @derive for explicit derives
//! - @auto for smart auto-derive
//! - Clone, Debug, PartialEq, Copy, Default

use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn compile_and_get_rust(code: &str) -> Result<String, String> {
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
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let generated_path = out_dir.join("test.rs");
    fs::read_to_string(&generated_path).map_err(|e| format!("Failed to read generated file: {}", e))
}

fn compile_and_verify(code: &str) -> (bool, String, String) {
    match compile_and_get_rust(code) {
        Ok(generated) => {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let rs_path = temp_dir.path().join("test.rs");
            fs::write(&rs_path, &generated).expect("Failed to write rs file");

            let rustc = Command::new("rustc")
                .arg("--crate-type=lib")
                .arg(&rs_path)
                .arg("-o")
                .arg(temp_dir.path().join("test.rlib"))
                .output();

            match rustc {
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr).to_string();
                    (output.status.success(), generated, err)
                }
                Err(e) => (false, generated, format!("Failed to run rustc: {}", e)),
            }
        }
        Err(e) => (false, String::new(), e),
    }
}

// ============================================================================
// @derive TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_clone() {
    let code = r#"
@derive(Clone)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn duplicate(p: &Point) -> Point {
    p.clone()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Clone"), "Should have Clone derive");
    assert!(success, "Derive Clone should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_debug() {
    let code = r#"
@derive(Debug)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn print_point(p: &Point) {
    println!("{:?}", p)
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Debug"), "Should have Debug derive");
    assert!(success, "Derive Debug should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_partial_eq() {
    let code = r#"
@derive(PartialEq)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn are_equal(a: &Point, b: &Point) -> bool {
    a == b
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(
        generated.contains("PartialEq"),
        "Should have PartialEq derive"
    );
    assert!(success, "Derive PartialEq should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_copy() {
    let code = r#"
@derive(Copy, Clone)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn use_copy(p: Point) -> (Point, Point) {
    (p, p)  // Copy allows using p twice
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Copy"), "Should have Copy derive");
    assert!(success, "Derive Copy should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_default() {
    let code = r#"
@derive(Default)
pub struct Config {
    width: i32,
    height: i32,
}

pub fn default_config() -> Config {
    Config::default()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Default"), "Should have Default derive");
    assert!(success, "Derive Default should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_multiple() {
    let code = r#"
@derive(Clone, Debug, PartialEq)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn test_all(a: &Point, b: &Point) -> bool {
    println!("{:?} == {:?}", a, b);
    a == b
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Clone"), "Should have Clone");
    assert!(generated.contains("Debug"), "Should have Debug");
    assert!(generated.contains("PartialEq"), "Should have PartialEq");
    assert!(success, "Multiple derives should compile. Error: {}", err);
}

// ============================================================================
// @auto TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_simple_struct() {
    let code = r#"
@auto
pub struct Point {
    x: i32,
    y: i32,
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    // @auto should derive common traits
    assert!(
        success,
        "@auto simple struct should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_with_string() {
    // String is not Copy, so @auto should not derive Copy
    let code = r#"
@auto
pub struct Person {
    name: string,
    age: i32,
}

pub fn clone_person(p: &Person) -> Person {
    p.clone()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Clone"), "Should derive Clone");
    // Should NOT derive Copy for String fields
    assert!(success, "@auto with String should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_with_vec() {
    // Vec is not Copy, so @auto should not derive Copy
    let code = r#"
@auto
pub struct Container {
    items: Vec<i32>,
}

pub fn clone_container(c: &Container) -> Container {
    c.clone()
}
"#;
    let (success, generated, err) = compile_and_verify(code);
    assert!(generated.contains("Clone"), "Should derive Clone");
    assert!(success, "@auto with Vec should compile. Error: {}", err);
}

// ============================================================================
// ENUM DERIVES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_enum() {
    let code = r#"
@derive(Clone, Debug, PartialEq)
pub enum Color {
    Red,
    Green,
    Blue,
}

pub fn is_red(c: &Color) -> bool {
    *c == Color::Red
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Enum derives should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_enum_with_data() {
    let code = r#"
@derive(Clone, Debug)
pub enum Message {
    Text(string),
    Number(i32),
}

pub fn clone_message(m: &Message) -> Message {
    m.clone()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Enum with data derives should compile. Error: {}",
        err
    );
}

// ============================================================================
// NESTED STRUCTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_nested() {
    let code = r#"
@derive(Clone, Debug)
pub struct Inner {
    value: i32,
}

@derive(Clone, Debug)
pub struct Outer {
    inner: Inner,
}

pub fn clone_outer(o: &Outer) -> Outer {
    o.clone()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Nested struct derives should compile. Error: {}",
        err
    );
}

// ============================================================================
// GENERIC STRUCT DERIVES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_derive_generic() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

pub fn clone_container<T: Clone>(c: &Container<T>) -> Container<T> {
    c.clone()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Generic struct derives should compile. Error: {}",
        err
    );
}

