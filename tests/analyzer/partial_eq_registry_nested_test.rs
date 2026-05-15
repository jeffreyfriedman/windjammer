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

//! TDD: `partial_eq_types` registry must match codegen so nested structs and enums
//! derive `PartialEq` when rustc needs it (fixes E0277 from missing trait impls).

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Inner has no `@auto`; codegen still derives PartialEq. Outer must compile with `==`.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_struct_partial_eq_registry() {
    let code = r#"
pub struct Inner {
    x: i32,
}

pub struct Outer {
    inner: Inner,
}

pub fn same() -> bool {
    let a = Outer { inner: Inner { x: 1 } };
    let b = Outer { inner: Inner { x: 1 } };
    a == b
}
"#;
    let (rs, ok) = test_utils::compile_single_check(code);
    let err = if !ok { &rs as &str } else { "" };
    assert!(
        rs.contains("PartialEq") && rs.contains("Inner") && rs.contains("Outer"),
        "Expected PartialEq on nested structs. Generated:\n{}",
        rs
    );
    assert!(
        !err.contains("E0277"),
        "Should not hit E0277 (PartialEq). rustc stderr:\n{}",
        err
    );
    assert!(ok, "rustc should accept generated Rust. stderr:\n{}", err);
}

/// Enum derives PartialEq only when payload types support it; custom struct must be registered.
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_enum_payload_struct_partial_eq_registry() {
    let code = r#"
pub struct Point {
    x: i32,
    y: i32,
}

pub enum Shape {
    Dot(Point),
    None,
}

pub fn same() -> bool {
    let a = Shape::Dot(Point { x: 0, y: 0 });
    let b = Shape::Dot(Point { x: 0, y: 0 });
    a == b
}
"#;
    let (rs, ok) = test_utils::compile_single_check(code);
    let err = if !ok { &rs as &str } else { "" };
    assert!(
        rs.contains("PartialEq") && rs.contains("Shape") && rs.contains("Point"),
        "Expected PartialEq on enum and struct. Generated:\n{}",
        rs
    );
    assert!(
        !err.contains("E0277"),
        "Should not hit E0277. stderr:\n{}",
        err
    );
    assert!(ok, "rustc should accept generated Rust. stderr:\n{}", err);
}
