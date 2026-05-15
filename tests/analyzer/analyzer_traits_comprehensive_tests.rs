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
//! Comprehensive Analyzer Trait/Impl Tests
//!
//! These tests verify that the Windjammer compiler correctly handles
//! trait definitions, implementations, and trait bounds.

#[path = "../common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

// ============================================================================
// BASIC IMPL BLOCKS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_basic() {
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
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Basic impl should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_multiple_methods() {
    let code = r#"
@derive(Clone, Debug)
pub struct Counter {
    value: i32,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { value: 0 }
    }
    
    pub fn increment(&mut self) {
        self.value += 1
    }
    
    pub fn get(&self) -> i32 {
        self.value
    }
    
    pub fn reset(&mut self) {
        self.value = 0
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Multiple methods impl should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_static_and_instance() {
    let code = r#"
@derive(Clone, Debug)
pub struct Config {
    value: i32,
}

impl Config {
    pub fn default_value() -> i32 {
        42
    }
    
    pub fn new() -> Config {
        Config { value: Config::default_value() }
    }
    
    pub fn get(&self) -> i32 {
        self.value
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Static and instance methods should compile. Error: {}",
        err
    );
}

// ============================================================================
// GENERIC IMPL BLOCKS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_generic() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

impl<T> Container<T> {
    pub fn new(value: T) -> Container<T> {
        Container { value: value }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Generic impl should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_generic_with_bound() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

impl<T: Clone> Container<T> {
    pub fn duplicate(&self) -> Container<T> {
        Container { value: self.value.clone() }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Generic impl with bound should compile. Error: {}",
        err
    );
}

// ============================================================================
// SELF PARAMETER VARIATIONS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_ref_self() {
    let code = r#"
@derive(Clone, Debug)
pub struct Data {
    value: i32,
}

impl Data {
    pub fn get(&self) -> i32 {
        self.value
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "&self method should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_mut_self() {
    let code = r#"
@derive(Clone, Debug)
pub struct Data {
    value: i32,
}

impl Data {
    pub fn set(&mut self, v: i32) {
        self.value = v
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "&mut self method should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_owned_self() {
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
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Owned self method should compile. Error: {}", err);
}

// ============================================================================
// ASSOCIATED FUNCTIONS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_associated_fn() {
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
    
    pub fn unit_x() -> Point {
        Point { x: 1, y: 0 }
    }
    
    pub fn unit_y() -> Point {
        Point { x: 0, y: 1 }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Associated functions should compile. Error: {}",
        err
    );
}

// ============================================================================
// ENUM IMPL
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_enum() {
    let code = r#"
@derive(Clone, Debug)
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn is_vertical(&self) -> bool {
        match self {
            Direction::Up => true,
            Direction::Down => true,
            Direction::Left => false,
            Direction::Right => false,
        }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Enum impl should compile. Error: {}", err);
}

// ============================================================================
// MULTIPLE IMPL BLOCKS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_multiple_impl_blocks() {
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

impl Point {
    pub fn get_x(&self) -> i32 {
        self.x
    }
    
    pub fn get_y(&self) -> i32 {
        self.y
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Multiple impl blocks should compile. Error: {}",
        err
    );
}

// ============================================================================
// METHOD RETURNING SELF TYPE
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_return_self_type() {
    let code = r#"
@derive(Clone, Debug)
pub struct Builder {
    value: i32,
}

impl Builder {
    pub fn new() -> Builder {
        Builder { value: 0 }
    }
    
    pub fn set(&mut self, v: i32) -> &mut Builder {
        self.value = v;
        self
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Return self type should compile. Error: {}", err);
}

// ============================================================================
// IMPL WITH DEFAULT VALUES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_impl_with_default() {
    let code = r#"
@derive(Clone, Debug, Default)
pub struct Config {
    width: i32,
    height: i32,
}

impl Config {
    pub fn new() -> Config {
        Config::default()
    }
    
    pub fn standard() -> Config {
        Config { width: 800, height: 600 }
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Impl with Default should compile. Error: {}", err);
}
