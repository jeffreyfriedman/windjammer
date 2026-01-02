//! Comprehensive Codegen Method Call Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust code for method calls, including:
//! - Self parameter handling (&self, &mut self, self)
//! - Borrowed parameters
//! - Owned parameters
//! - Method chaining

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
// &self METHODS
// ============================================================================

#[test]
fn test_method_ref_self() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn get(&self) -> i32 {
        self.value
    }
}

pub fn use_counter(c: &Counter) -> i32 {
    c.get()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "&self method should compile. Error: {}", err);
}

#[test]
fn test_method_ref_self_chain() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn get(&self) -> i32 {
        self.value
    }
    
    pub fn double(&self) -> i32 {
        self.get() * 2
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "&self chain should compile. Error: {}", err);
}

// ============================================================================
// &mut self METHODS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_mut_self() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn increment(&mut self) {
        self.value += 1
    }
}

pub fn use_counter(c: &mut Counter) {
    c.increment()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "&mut self method should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_mut_self_returns_self() {
    let code = r#"
@derive(Clone, Debug)
pub struct Builder {
    value: i32,
}

impl Builder {
    pub fn set(&mut self, v: i32) -> &mut Builder {
        self.value = v;
        self
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "&mut self returning self should compile. Error: {}",
        err
    );
}

// ============================================================================
// CONSUMING self METHODS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_consuming_self() {
    let code = r#"
@derive(Clone, Debug)
pub struct Wrapper {
    value: i32,
}

impl Wrapper {
    pub fn unwrap(self) -> i32 {
        self.value
    }
}

pub fn use_wrapper(w: Wrapper) -> i32 {
    w.unwrap()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Consuming self should compile. Error: {}", err);
}

// ============================================================================
// STATIC METHODS (NO SELF)
// ============================================================================

#[test]
fn test_static_method() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn origin() -> Point {
        Point { x: 0, y: 0 }
    }
}

pub fn get_origin() -> Point {
    Point::origin()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Static method should compile. Error: {}", err);
}

#[test]
fn test_static_method_with_params() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point { x: x, y: y }
    }
}

pub fn create_point() -> Point {
    Point::new(10, 20)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Static method with params should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHOD WITH BORROWED PARAMS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_borrowed_param() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container {
    items: Vec<i32>,
}

impl Container {
    pub fn contains(&self, value: &i32) -> bool {
        self.items.contains(value)
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Method with borrowed param should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHOD WITH OWNED PARAMS
// ============================================================================

#[test]
fn test_method_owned_param() {
    let code = r#"
@derive(Clone, Debug)
pub struct Item {
    name: string,
}

@derive(Clone, Debug)
pub struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn add(&mut self, item: Item) {
        self.items.push(item)
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Method with owned param should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHOD CHAINING
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_chaining_vec() {
    let code = r#"
pub fn chain_example(items: &Vec<i32>) -> i32 {
    items.iter().filter(|x| **x > 0).count() as i32
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Method chaining on Vec should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_chaining_string() {
    let code = r#"
pub fn chain_string(s: &string) -> string {
    s.trim().to_uppercase()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Method chaining on String should compile. Error: {}",
        err
    );
}

// ============================================================================
// CALLING METHODS ON FIELDS
// ============================================================================

#[test]
fn test_method_on_field() {
    let code = r#"
@derive(Clone, Debug)
pub struct Inner {
    value: i32,
}

impl Inner {
    pub fn get(&self) -> i32 {
        self.value
    }
}

@derive(Clone, Debug)
pub struct Outer {
    inner: Inner,
}

impl Outer {
    pub fn get_inner_value(&self) -> i32 {
        self.inner.get()
    }
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Method on field should compile. Error: {}", err);
}

// ============================================================================
// GENERIC METHODS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_method_call() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

impl<T: Clone> Container<T> {
    pub fn get(&self) -> T {
        self.value.clone()
    }
}

pub fn use_container(c: &Container<i32>) -> i32 {
    c.get()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Generic method call should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHODS RETURNING REFERENCES
// ============================================================================

#[test]
fn test_method_returning_ref() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container {
    value: i32,
}

impl Container {
    pub fn value_ref(&self) -> &i32 {
        &self.value
    }
}

pub fn get_ref(c: &Container) -> i32 {
    *c.value_ref()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Method returning ref should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHODS WITH MULTIPLE PARAMS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_multiple_params() {
    let code = r#"
@derive(Clone, Debug)
pub struct Calculator {
    base: i32,
}

impl Calculator {
    pub fn add_multiply(&self, a: i32, b: i32) -> i32 {
        self.base + a * b
    }
}

pub fn use_calc(c: &Calculator) -> i32 {
    c.add_multiply(2, 3)
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(
        success,
        "Method with multiple params should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHODS ON PRIMITIVES (VIA STDLIB)
// ============================================================================

#[test]
fn test_method_on_i32() {
    let code = r#"
pub fn abs_value(n: i32) -> i32 {
    n.abs()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Method on i32 should compile. Error: {}", err);
}

#[test]
fn test_method_on_string() {
    let code = r#"
pub fn string_len(s: &string) -> usize {
    s.len()
}
"#;
    let (success, _generated, err) = compile_and_verify(code);
    assert!(success, "Method on string should compile. Error: {}", err);
}
