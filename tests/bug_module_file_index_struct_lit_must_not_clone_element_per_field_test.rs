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

//! Regression: module file index struct lit must not clone element per field.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Pass {
    pub id: i32,
    pub bindings: Vec<i32>,
    pub dispatch_x: u32,
}

pub fn copy_pass(passes: Vec<Pass>, i: usize) -> Pass {
    Pass {
        id: passes[i].id,
        bindings: passes[i].bindings,
        dispatch_x: passes[i].dispatch_x,
    }
}
"#;

#[test]
fn module_file_index_struct_lit_must_not_clone_element_per_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().id")
        || rs.contains("].clone().bindings")
        || rs.contains("].clone().dispatch_x");
    assert!(
        !bad,
        "RED: per-field indexed element clone in struct lit:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
