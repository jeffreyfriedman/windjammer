#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-402: Copy `HostType::F32` / `ShaderType::F32` must not emit `.clone()`.
//!
//! WDB-384/392/397/401 cover FaceDirection / Direction / TileType / StreamState;
//! product gen/rendering/type_safety_validator.rs still has
//! `HostType::F32.clone()` and `ShaderType::F32.clone()`.
//! WJ source is `HostType::F32` / `ShaderType::F32` in UniformBinding::new.
//! Numbered 402 because 395–400 are reserved or used by the compiler agent.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum HostType {
    F32,
    U32,
}

pub enum ShaderType {
    F32,
    U32,
}

pub struct Binding {
    pub host: HostType,
    pub shader: ShaderType,
}

pub fn screen_width() -> Binding {
    Binding {
        host: HostType::F32,
        shader: ShaderType::F32,
    }
}

pub fn sample_count() -> Binding {
    Binding {
        host: HostType::U32,
        shader: ShaderType::U32,
    }
}
"#;

#[test]
fn module_file_hosttype_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("HostType::F32.clone()")
        || rs.contains("HostType::U32.clone()")
        || rs.contains("ShaderType::F32.clone()")
        || rs.contains("ShaderType::U32.clone()");
    assert!(
        !bad,
        "Copy HostType/ShaderType unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
