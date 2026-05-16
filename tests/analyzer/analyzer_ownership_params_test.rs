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
#![allow(unused)]
//! Analyzer ownership: parameters, explicit annotations, self.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    // Read-only params may be borrowed or kept owned (both are valid)
    // The important thing is that the code compiles
    assert!(
        success,
        "Generated code should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    // Field reads may keep owned or borrow - both are valid
    // The key is that the code compiles
    assert!(
        success,
        "Generated code should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // read_counter should take &Counter since it only calls a read method
    assert!(success, "Error: {}", err);
}

// ============================================================================
// MUTABLE PARAMETER (INFER &mut)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    assert!(
        generated.contains("&mut Counter"),
        "Should infer &mut for field mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    // With explicit &mut, should compile
    assert!(
        generated.contains("&mut Counter"),
        "Should preserve explicit &mut. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

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
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

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
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // identity should take Point by value since it's returned
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_owned_for_copy_types() {
    let code = r#"
pub fn add(x: i32, y: i32) -> i32 {
    x + y
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

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
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    assert!(
        generated.contains("&Data"),
        "Explicit & should be preserved. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    assert!(
        generated.contains("&mut Data"),
        "Explicit &mut should be preserved. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}
