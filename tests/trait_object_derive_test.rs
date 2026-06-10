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

//! Tests for trait object handling in auto-derive and codegen.
//!
//! Bug: `Vec<trait Plugin>` (Windjammer syntax) is parsed as `Vec<ImplTrait("Plugin")>`,
//! but `type_contains_trait_object` only checks `Type::TraitObject`, not `Type::ImplTrait`.
//! This causes Debug/Clone to be incorrectly derived for structs with trait object fields.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_struct_with_vec_impl_trait_field_skips_debug_clone_derive() {
    let source = r#"
pub trait Plugin {
    fn name(self) -> string
}

pub struct App {
    plugins: Vec<trait Plugin>,
    count: i32,
}
"#;
    let output = test_utils::compile_single(source);

    assert!(!output.is_empty(), "Compilation should produce output");
    assert!(
        !output.contains("#[derive(Debug, Clone"),
        "App should NOT derive Debug/Clone because it contains Vec<trait Plugin> \
         (trait X in struct fields = trait object = not Debug/Clone).\n\
         Generated:\n{}",
        output
    );
}

#[test]
fn test_struct_with_direct_impl_trait_field_skips_debug_clone_derive() {
    let source = r#"
pub trait Renderer {
    fn render(self)
}

pub struct Scene {
    renderer: trait Renderer,
}
"#;
    let output = test_utils::compile_single(source);

    assert!(!output.is_empty(), "Compilation should produce output");
    assert!(
        !output.contains("#[derive(Debug, Clone"),
        "Scene should NOT derive Debug/Clone because it contains `trait Renderer` field.\n\
         Generated:\n{}",
        output
    );
}

#[test]
fn test_struct_without_trait_fields_still_derives_debug_clone() {
    let source = r#"
pub struct Point {
    x: f32,
    y: f32,
}
"#;
    let output = test_utils::compile_single(source);

    assert!(
        output.contains("Debug") && output.contains("Clone"),
        "Point should derive Debug and Clone since it has no trait object fields.\n\
         Generated:\n{}",
        output
    );
}
