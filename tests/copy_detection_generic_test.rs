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

//! Generic Copy Detection Tests
//!
//! Verifies that Copy detection works generically from @derive(Copy) in source,
//! NOT from hardcoded type names. The compiler must NEVER know about
//! application-specific types.
//!
//! Architecture: copy_structs_registry (PASS 0) collects types with @derive(Copy)
//! or all-Copy fields. is_known_copy_type is ONLY for external crate types.

#[path = "common/test_utils.rs"]
mod test_utils;

/// Custom struct with @derive(Copy) - should NOT generate *(data)
#[test]
fn test_custom_copy_type_from_derive() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct MyData { value: i32 }

pub fn process(data: MyData) -> i32 {
    data.value
}

pub fn main() {
    let d = MyData { value: 42 };
    let _ = process(d);
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(data)"),
        "Should NOT add *(data) for Copy MyData. Generated:\n{}",
        rs
    );
}

/// Struct without @derive(Copy) - should handle correctly
#[test]
fn test_non_copy_type_no_derive() {
    let source = r#"
pub struct MyData { value: String }

pub fn process(data: &MyData) -> usize {
    data.value.len()
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

/// Struct with all-Copy fields (no explicit derive) - fixed-point should discover it
#[test]
fn test_implicit_copy_all_primitive_fields() {
    let source = r#"
pub struct Point3D { x: f32, y: f32, z: f32 }

pub fn distance(p: Point3D) -> f32 {
    (p.x * p.x + p.y * p.y + p.z * p.z).sqrt()
}

pub fn main() {
    let p = Point3D { x: 1.0, y: 2.0, z: 3.0 };
    let _ = distance(p);
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

/// Option<CopyType> in if let - should NOT add wrongful *
#[test]
fn test_option_copy_type_if_let() {
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct State { id: u32 }

pub struct Container { state: Option<State> }

impl Container {
    pub fn new() -> Container {
        Container { state: None }
    }
    fn update(self) {
        if let Some(s) = self.state {
            let _ = s.id;
        }
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(s)"),
        "Should NOT add *(s) for Copy State. Generated:\n{}",
        rs
    );
}

/// Nested Copy struct - both should be in registry
#[test]
fn test_nested_copy_structs() {
    let source = r#"
@derive(Copy, Clone)
pub struct Vec2 { x: f32, y: f32 }

@derive(Copy, Clone)
pub struct Transform { pos: Vec2, scale: f32 }

pub fn get_x(t: Transform) -> f32 {
    t.pos.x
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}

/// Unit enum - always Copy
#[test]
fn test_unit_enum_copy() {
    let source = r#"
pub enum Direction { North, South, East, West }

pub fn is_north(d: Direction) -> bool {
    match d {
        Direction::North => true,
        _ => false,
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
}
