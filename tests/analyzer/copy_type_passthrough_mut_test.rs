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

//! TDD Test: Copy-type parameters must also use passthrough inference
//!
//! Bug: When a Copy-type parameter is passed to a function expecting &mut,
//! the Copy-type shortcut (analyzer/mod.rs lines 2016-2023) only checks
//! is_mutated() (direct field mutation). It never calls infer_passthrough_ownership,
//! so the parameter defaults to Owned — silently losing the mutation.
//!
//! Example:
//!   struct Point { x: f32, y: f32 }  // Copy (all fields Copy)
//!   fn shift(p: Point) { p.x = p.x + 1.0 }  // is_mutated → &mut Point ✓
//!   fn process(p: Point) { shift(p) }  // passthrough → should be &mut Point ✗ (gets Owned)
//!
//! Root Cause: Copy types take a shortcut that skips passthrough inference.
//! Fix: Check passthrough BEFORE defaulting Copy types to Owned.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_copy_type_passthrough_to_mutating_fn() {
    let code = r#"
pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub fn shift_right(p: Point) {
    p.x = p.x + 1.0
}

pub fn process(p: Point) {
    shift_right(p)
}
"#;

    let rust = test_utils::compile_single_result(code).expect("Should compile");

    assert!(
        rust.contains("p: &mut Point"),
        "shift_right should infer p: &mut Point (direct mutation). Got:\n{}",
        rust
    );

    // THE BUG: process() passes p to shift_right which expects &mut Point,
    // but Copy shortcut defaults to Owned without checking passthrough.
    assert!(
        rust.contains("pub fn process(p: &mut Point"),
        "process() should infer p: &mut Point via passthrough to shift_right.\n\
         Copy-type shortcut skips passthrough inference.\n\
         Got:\n{}",
        rust
    );
    test_utils::verify_rust_compiles(&rust).expect("Generated Rust should compile");
}

#[test]
fn test_copy_type_cross_module_passthrough() {
    let files = &[
        (
            "math.wj",
            r#"
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub fn normalize(v: Vec2) {
    let len = (v.x * v.x + v.y * v.y).sqrt()
    if len > 0.0 {
        v.x = v.x / len
        v.y = v.y / len
    }
}
"#,
        ),
        (
            "game.wj",
            r#"
use crate::Vec2
use crate::math

pub fn update_direction(dir: Vec2) {
    math::normalize(dir)
}
"#,
        ),
    ];

    let results = test_utils::compile_project_result(files).expect("Should compile");
    let game_rs = results.get("game.rs").expect("game.rs should be generated");

    assert!(
        game_rs.contains("dir: &mut Vec2"),
        "update_direction should infer dir: &mut Vec2 via cross-module passthrough.\n\
         Copy types must also use passthrough inference.\n\
         Got:\n{}",
        game_rs
    );
}

#[test]
fn test_copy_type_readonly_stays_owned() {
    // Negative test: Copy type passed to a readonly function should remain Owned
    let code = r#"
pub struct Point {
    pub x: f32,
    pub y: f32,
}

pub fn magnitude(p: Point) -> f32 {
    (p.x * p.x + p.y * p.y).sqrt()
}

pub fn process(p: Point) -> f32 {
    magnitude(p)
}
"#;

    let rust = test_utils::compile_single_result(code).expect("Should compile");

    // Both should stay Owned (pass by value) since magnitude only reads
    assert!(
        !rust.contains("p: &mut Point") || rust.contains("p: Point"),
        "Readonly Copy passthrough should NOT upgrade to &mut.\n\
         Got:\n{}",
        rust
    );
}

#[test]
fn test_copy_type_self_field_passthrough_mut() {
    // Self.field where field is Copy type, passed to mutating function
    let code = r#"
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
}

pub fn apply_rotation(t: Transform) {
    t.rotation = t.rotation + 0.1
}

pub struct Player {
    pub transform: Transform,
    pub name: string,
}

impl Player {
    pub fn update(self) {
        apply_rotation(self.transform)
    }
}
"#;

    let rust = test_utils::compile_single_result(code).expect("Should compile");

    assert!(
        rust.contains("t: &mut Transform"),
        "apply_rotation should infer t: &mut Transform. Got:\n{}",
        rust
    );
    assert!(
        rust.contains("pub fn update(&mut self"),
        "Player.update should infer &mut self when passing self.transform to mutating fn.\n\
         Got:\n{}",
        rust
    );
    test_utils::verify_rust_compiles(&rust).expect("Generated Rust should compile");
}
