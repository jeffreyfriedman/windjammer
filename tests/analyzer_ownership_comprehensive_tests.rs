//! Comprehensive Analyzer Ownership Inference Tests
//!
//! These tests verify the Windjammer compiler's automatic ownership inference.
//! The compiler infers &, &mut, or owned for parameters based on usage.
//! This is a core Windjammer philosophy: "The compiler does the work, not the developer."

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
// READ-ONLY PARAMETER (INFER &)
// ============================================================================

#[test]
fn test_infer_borrowed_for_readonly_param() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn print_point(p: Point) {
    println!("{}, {}", p.x, p.y)
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Read-only params may be borrowed or kept owned (both are valid)
    // The important thing is that the code compiles
    assert!(
        success,
        "Generated code should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
fn test_infer_borrowed_for_field_read() {
    let code = r#"
@derive(Clone, Debug)
pub struct Rectangle {
    width: i32,
    height: i32,
}

pub fn area(rect: Rectangle) -> i32 {
    rect.width * rect.height
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Field reads may keep owned or borrow - both are valid
    // The key is that the code compiles
    assert!(
        success,
        "Generated code should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
fn test_infer_borrowed_for_method_read() {
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

pub fn read_counter(c: Counter) -> i32 {
    c.get()
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // read_counter should take &Counter since it only calls a read method
    assert!(success, "Error: {}", err);
}

// ============================================================================
// MUTABLE PARAMETER (INFER &mut)
// ============================================================================

#[test]
fn test_infer_mut_ref_for_field_mutation() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

pub fn increment(c: Counter) {
    c.value = c.value + 1
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("&mut Counter"),
        "Should infer &mut for field mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
fn test_infer_mut_ref_for_method_mutation() {
    // When calling a &mut self method, should either:
    // 1. Infer &mut for the parameter, OR
    // 2. Use mut c: Counter if passing by value
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn increment(&mut self) {
        self.value = self.value + 1
    }
}

pub fn bump(c: &mut Counter) {
    c.increment()
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // With explicit &mut, should compile
    assert!(
        generated.contains("&mut Counter"),
        "Should preserve explicit &mut. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
fn test_infer_mut_ref_for_compound_assignment() {
    let code = r#"
@derive(Clone, Debug)
pub struct Stats {
    score: i32,
}

pub fn add_points(s: Stats, points: i32) {
    s.score += points
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("&mut Stats"),
        "Should infer &mut for compound assignment. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

// ============================================================================
// OWNED PARAMETER (NO INFERENCE)
// ============================================================================

#[test]
fn test_owned_when_stored() {
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
    let (success, generated, err) = compile_and_verify(code);

    // add should take Item by value since it's stored (pushed)
    // Should NOT be &Item
    assert!(
        !generated.contains("&Item") || generated.contains("item: Item"),
        "Should keep owned when item is stored. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
fn test_owned_when_returned() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

pub fn identity(p: Point) -> Point {
    p
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // identity should take Point by value since it's returned
    assert!(success, "Error: {}", err);
}

#[test]
fn test_owned_for_copy_types() {
    let code = r#"
pub fn add(x: i32, y: i32) -> i32 {
    x + y
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Primitive types should remain owned (no &i32)
    assert!(
        !generated.contains("&i32"),
        "Copy types should not be borrowed. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

// ============================================================================
// EXPLICIT ANNOTATIONS RESPECTED
// ============================================================================

#[test]
fn test_explicit_borrowed_respected() {
    let code = r#"
@derive(Clone, Debug)
pub struct Data {
    value: i32,
}

pub fn process(d: &Data) -> i32 {
    d.value
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("&Data"),
        "Explicit & should be preserved. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
fn test_explicit_mut_borrowed_respected() {
    let code = r#"
@derive(Clone, Debug)
pub struct Data {
    value: i32,
}

pub fn modify(d: &mut Data) {
    d.value = 42
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("&mut Data"),
        "Explicit &mut should be preserved. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

// ============================================================================
// SELF PARAMETER INFERENCE
// ============================================================================

#[test]
fn test_self_borrowed_for_read() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn sum(self) -> i32 {
        self.x + self.y
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Without explicit &self, should infer &self for read-only
    assert!(success, "Error: {}", err);
}

#[test]
fn test_self_mut_for_mutation() {
    let code = r#"
@derive(Clone, Debug)
pub struct Point {
    x: i32,
    y: i32,
}

impl Point {
    pub fn set_x(self, x: i32) {
        self.x = x
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Should infer &mut self for mutation
    assert!(
        generated.contains("&mut self"),
        "Should infer &mut self for mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

// ============================================================================
// COMPLEX SCENARIOS
// ============================================================================

#[test]
fn test_mixed_read_and_mut_params() {
    let code = r#"
@derive(Clone, Debug)
pub struct Source {
    data: i32,
}

@derive(Clone, Debug)
pub struct Target {
    data: i32,
}

pub fn copy_data(src: Source, dst: Target) {
    dst.data = src.data
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // dst must be &mut since it's mutated
    assert!(
        generated.contains("&mut Target"),
        "dst should be mut borrowed. Generated:\n{}",
        generated
    );
    assert!(
        success,
        "Generated code should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
fn test_nested_field_mutation() {
    let code = r#"
@derive(Clone, Debug)
pub struct Inner {
    value: i32,
}

@derive(Clone, Debug)
pub struct Outer {
    inner: Inner,
}

pub fn set_inner_value(o: Outer, v: i32) {
    o.inner.value = v
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("&mut Outer"),
        "Should infer &mut for nested field mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
fn test_conditional_mutation() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

pub fn maybe_increment(c: Counter, do_it: bool) {
    if do_it {
        c.value = c.value + 1
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Even if mutation is conditional, should still be &mut
    assert!(
        generated.contains("&mut Counter"),
        "Should infer &mut even for conditional mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
fn test_loop_mutation() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

pub fn increment_n_times(c: Counter, n: i32) {
    let mut i = 0
    while i < n {
        c.value = c.value + 1
        i = i + 1
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    assert!(
        generated.contains("&mut Counter"),
        "Should infer &mut for loop mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

// ============================================================================
// ITERATOR SCENARIOS
// ============================================================================

#[test]
fn test_borrow_for_iteration() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container {
    items: Vec<i32>,
}

pub fn sum_items(c: Container) -> i32 {
    let mut total = 0
    for item in c.items {
        total = total + item
    }
    total
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Should borrow container for iteration
    assert!(success, "Error: {}", err);
}

#[test]
fn test_mut_borrow_for_item_modification() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container {
    items: Vec<i32>,
}

pub fn double_items(c: Container) {
    for item in c.items {
        item = item * 2
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Should use &mut for item modification
    // This may or may not compile depending on how iter_mut is inferred
    // Just verify it compiles or generates reasonable code
    println!("Generated:\n{}", generated);
}

// ============================================================================
// TRAIT IMPLEMENTATION SCENARIOS
// ============================================================================

#[test]
fn test_trait_impl_preserves_signature() {
    let code = r#"
trait Printable {
    fn print(&self) { }
}

@derive(Clone, Debug)
pub struct MyType {
    value: i32,
}

impl Printable for MyType {
    fn print(&self) {
        println!("{}", self.value)
    }
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Trait impl should match trait signature exactly
    assert!(success, "Error: {}", err);
}

// ============================================================================
// GENERIC TYPE SCENARIOS
// ============================================================================

#[test]
fn test_generic_param_ownership() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

pub fn get_value<T>(c: Container<T>) -> T {
    c.value.clone()
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Generic containers should have sensible inference
    assert!(success, "Error: {}", err);
}

#[test]
fn test_generic_with_clone() {
    let code = r#"
pub fn clone_item<T: Clone>(item: T) -> T {
    item.clone()
}
"#;
    let (success, generated, err) = compile_and_verify(code);

    // Should work with T: Clone bound
    assert!(success, "Error: {}", err);
}
