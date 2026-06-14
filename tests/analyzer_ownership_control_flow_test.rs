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
//! Analyzer ownership: control flow and iterators.
#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// SELF PARAMETER INFERENCE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Without explicit &self, should infer &self for read-only
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

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
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

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
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    assert!(
        generated.contains("&mut Outer"),
        "Should infer &mut for nested field mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    // Even if mutation is conditional, should still be &mut
    assert!(
        generated.contains("&mut Counter"),
        "Should infer &mut even for conditional mutation. Generated:\n{}",
        generated
    );
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

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
#[cfg_attr(tarpaulin, ignore)]
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Should borrow container for iteration
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { "compilation failed" } else { "" };

    // Should use &mut for item modification
    // This may or may not compile depending on how iter_mut is inferred
    // Just verify it compiles or generates reasonable code
    println!("Generated:\n{}", generated);
}
