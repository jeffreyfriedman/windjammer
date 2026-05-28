#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

/// TDD Test: Auto-derive Copy for data-carrying enums when all fields are Copy
///
/// Bug: The compiler only derives Copy for unit-only enums, but should also
/// derive Copy for data-carrying enums whose variant fields are all Copy types.
///
/// Example:
///   enum Shape {
///       Circle { radius: f32 },
///       Rectangle { width: f32, height: f32 },
///   }
///   // Should derive Copy because all fields (f32) are Copy
///
///   enum Event {
///       Message { text: String },
///   }
///   // Should NOT derive Copy because String is not Copy
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_enum_all_copy_fields_gets_copy_derive() {
    let source = r#"
enum Shape {
    Circle { radius: f32, x: f32, y: f32 },
    Rectangle { x: f32, y: f32, width: f32, height: f32 },
    Point { x: f32, y: f32 },
}

fn main() {
    let s = Shape::Circle { radius: 5.0, x: 0.0, y: 0.0 }
    println!("ok")
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("Copy"),
        "Enum with only f32 fields should derive Copy.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_enum_with_string_field_no_copy_derive() {
    let source = r#"
enum Event {
    Message { text: String },
    Score { value: i32 },
    Quit,
}

fn main() {
    let e = Event::Quit
    println!("ok")
}
"#;
    let rust = test_utils::compile_single(source);
    // Should NOT have Copy because String is not Copy
    // But should still have Clone
    assert!(
        !rust.contains("Copy"),
        "Enum with String field should NOT derive Copy.\nGenerated:\n{}",
        rust
    );
    assert!(
        rust.contains("Clone"),
        "Enum with String field should still derive Clone.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_enum_mixed_unit_and_data_all_copy() {
    let source = r#"
enum AIState {
    Idle,
    Patrol { waypoint: i32 },
    Chase { target_id: i32 },
    Attack { target_id: i32, cooldown: f32 },
    Flee { from_x: f32, from_y: f32 },
    Dead,
}

fn main() {
    let s = AIState::Idle
    println!("ok")
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("Copy"),
        "Enum mixing unit and data variants with only i32/f32 fields should derive Copy.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_enum_with_vec_field_no_copy() {
    let source = r#"
enum Container {
    Single { value: f32 },
    Multiple { values: Vec<f32> },
}

fn main() {
    let c = Container::Single { value: 1.0 }
    println!("ok")
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        !rust.contains("Copy"),
        "Enum with Vec field should NOT derive Copy.\nGenerated:\n{}",
        rust
    );
}

#[test]
fn test_unit_only_enum_still_gets_copy() {
    let source = r#"
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn main() {
    let d = Direction::Up
    println!("ok")
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("Copy"),
        "Unit-only enum should still derive Copy.\nGenerated:\n{}",
        rust
    );
}
