//! Partial `@derive(Clone)` / `@auto(Clone)` must still merge inferred traits (Debug, Copy, …).
//!
//! Dogfooding: game structs often use `@derive(Clone)` only; parent enums use `#[derive(Debug)]`
//! and require nested structs to implement `Debug`. Merging explicit derives with inference
//! fixes E0277 (`T doesn't implement Debug`).

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_partial_derive_clone_merges_debug_for_nested_enum() {
    let source = r#"
@derive(Clone)
pub struct Inner {
    x: i32,
}

pub enum Outer {
    V(Inner),
}

pub fn main() {
    let i = Inner { x: 1 }
    let o = Outer::V(i)
    println!("{:?}", o)
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Nested enum Debug requires Inner: Debug. rustc failed. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("derive(") && rs.contains("Debug") && rs.contains("Clone"),
        "Expected merged derive(Debug, Clone, ...) on Inner, got:\n{}",
        rs
    );
}

#[test]
fn test_partial_auto_clone_merges_debug() {
    let source = r#"
@auto(Clone)
pub struct Inner {
    x: i32,
}

pub fn main() {
    let i = Inner { x: 1 }
    println!("{:?}", i)
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "println! Debug requires Inner: Debug. rustc failed. Generated:\n{}",
        rs
    );
    assert!(
        rs.contains("Debug") && rs.contains("Clone"),
        "Expected merged derive on Inner, got:\n{}",
        rs
    );
}
