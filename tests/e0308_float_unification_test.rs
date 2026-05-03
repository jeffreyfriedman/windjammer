//! TDD: E0308 f32/f64 mismatches at API boundaries (dogfooding game-core).
//!
//! Patterns from session log:
//! - `ffi::api::fn(...)` — nested `FieldAccess` prevented signature lookup → literals stayed f64.
//! - Nested `mod` functions were not registered in float inference.
//! - `if` / `else` with `8.0` vs `x as f32` — cast result was not constrained as f32 for branch unification.
//! - Unknown float literals defaulted to `_f64` in codegen.

#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_nested_mod_call_registers_signature_and_literals_are_f32() {
    let source = r#"
mod ffi {
    mod api {
        pub fn tilemap_check_collision(
            tilemap_id: u32,
            x: f32,
            y: f32,
            width: f32,
            height: f32,
            tile_size: f32,
        ) -> bool {
            false
        }
    }
}

pub fn demo() -> bool {
    ffi::api::tilemap_check_collision(0u32, 0.0, 0.0, 1.0, 1.0, 1.0)
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("1.0_f32") || output.contains("1.0f32"),
        "unsuffixed 1.0 args to f32 params should be f32, got:\n{}",
        output
    );
    assert!(
        !output.contains("1.0_f64"),
        "should not emit 1.0_f64 for f32 FFI-style params, got:\n{}",
        output
    );
}

#[test]
fn test_if_else_float_literal_unifies_with_as_f32_branches() {
    let source = r#"
pub fn primitive_scale(node_type: i32) -> f32 {
    if node_type == 2 {
        8.0
    } else {
        if node_type >= 3 {
            node_type as f32
        } else {
            (node_type + 1) as f32
        }
    }
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("8.0_f32") || output.contains("8.0f32"),
        "then-branch literal should unify to f32 with as f32 else arms, got:\n{}",
        output
    );
    assert!(
        !output.contains("8.0_f64"),
        "then-branch must not stay f64, got:\n{}",
        output
    );
}

#[test]
fn test_unknown_float_inference_defaults_to_f32_suffix() {
    let source = r#"
pub fn bare() -> i32 {
    let _x = 2.5
    0
}
"#;
    let output = test_utils::compile_single(source);
    assert!(
        output.contains("2.5_f32") || output.contains("2.5f32"),
        "unconstrained literal should default to f32, got:\n{}",
        output
    );
}
