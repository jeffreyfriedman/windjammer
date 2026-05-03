//! Comprehensive @auto/@derive Tests
//!
//! These tests verify that the Windjammer compiler correctly generates
//! Rust derive attributes, including:
//! - @derive for explicit derives
//! - @auto for smart auto-derive
//! - Clone, Debug, PartialEq, Copy, Default

#[path = "test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
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
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Generic struct derives should compile. Error: {}",
        err
    );
}
