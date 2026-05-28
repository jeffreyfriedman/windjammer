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

//! E0308 Phase 9: Verify Pattern A - Struct literal tuple float fields
//!
//! Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) } with rotation: (f32, f32, f32, f32)
//! should generate (0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32)

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_struct_tuple_field_f32() {
    let source = r#"
pub struct Keyframe {
    pub rotation: (f32, f32, f32, f32),
    pub scale: (f32, f32, f32),
}

pub fn default_keyframe() -> Keyframe {
    Keyframe {
        rotation: (0.0, 0.0, 0.0, 1.0),
        scale: (1.0, 1.0, 1.0),
    }
}
"#;

    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "Expected f32 literals in tuple. Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("_f64"),
        "Tuple fields should infer f32 from struct. Got:\n{}",
        rust
    );
}
