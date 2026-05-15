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

//! TDD: Structs that contain a custom field whose definition includes a trait object
//! must not auto-derive `Debug`/`Clone` (Rust cannot derive them for `Box<dyn Trait>`).
//!
//! Regression: `Type::Custom` in `type_contents_trait_object` returned false, so only the
//! inner struct skipped derives while the outer struct still got `#[derive(Debug, Clone)]`.

#[path = "../common/test_utils.rs"]
mod test_utils;

/// Last `#[derive(...)]` attribute whose start appears before `byte_idx` in `rust`.
fn last_derive_before(rust: &str, byte_idx: usize) -> Option<&str> {
    let prev = &rust[..byte_idx];
    let dpos = prev.rfind("#[derive(")?;
    let rest = &prev[dpos..];
    let end = rest.find("]\n").or_else(|| rest.find(']'))?;
    Some(&rest[..=end])
}

fn assert_no_debug_clone_derive_immediately_before_struct(rust: &str, struct_name: &str) {
    let needle = format!("pub struct {}", struct_name);
    let idx = rust
        .find(&needle)
        .unwrap_or_else(|| panic!("missing struct {}", struct_name));
    match last_derive_before(rust, idx) {
        None => {}
        Some(attr) => assert!(
            !(attr.contains("Debug") && attr.contains("Clone")),
            "struct {} must not have #[derive(Debug, Clone, ...)]; found: {}",
            struct_name,
            attr
        ),
    }
}

#[test]
fn test_outer_struct_skips_debug_clone_when_inner_has_trait_object_field() {
    let source = r#"
pub trait RenderSystem {
    fn tick(self)
}

pub struct RenderSystemManager {
    systems: Vec<trait RenderSystem>,
}

pub struct VoxelGPURenderer {
    manager: RenderSystemManager,
}
"#;
    let output = test_utils::compile_single(source);
    assert!(!output.is_empty(), "expected generated Rust");

    assert_no_debug_clone_derive_immediately_before_struct(&output, "RenderSystemManager");
    assert_no_debug_clone_derive_immediately_before_struct(&output, "VoxelGPURenderer");
}

#[test]
fn test_simple_struct_still_derives_debug_clone() {
    let source = r#"
pub struct Point {
    x: f32,
    y: f32,
}
"#;
    let output = test_utils::compile_single(source);
    let needle = "pub struct Point";
    let idx = output.find(needle).expect("Point struct");
    let attr = last_derive_before(&output, idx).expect("derive before Point");
    assert!(
        attr.contains("Debug") && attr.contains("Clone"),
        "all-primitive struct should still derive Debug and Clone; got: {}",
        attr
    );
}
