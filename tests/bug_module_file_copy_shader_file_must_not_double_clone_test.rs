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

//! Regression: module file copy shader file must not double clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum ShaderFile {
    A,
    B,
}

pub struct Pass {
    pub shader_file: ShaderFile,
}

pub fn first(passes: Vec<Pass>) -> ShaderFile {
    passes[0].shader_file
}
"#;

#[test]
fn module_file_copy_shader_file_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().shader_file") || rs.contains(".shader_file.clone()");
    assert!(
        !bad,
        "RED: indexed Copy ShaderFile cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
