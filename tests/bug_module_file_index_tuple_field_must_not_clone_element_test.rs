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

//! indexed tuple/Copy field access must not clone the whole element.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Xform {
    pub rotation: (f32, f32, f32, f32),
}

pub fn rot0(xs: Vec<Xform>, idx: usize) -> f32 {
    xs[idx].rotation.0
}
"#;

#[test]
fn module_file_index_tuple_field_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone()");
    assert!(
        !bad,
        "RED indexed tuple field cloned element:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
