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

//! `match expr.clone()` must not clone when matching by value is unnecessary.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Shape {
    Sphere(f32),
    Box,
}

pub struct Body {
    pub shape: Shape,
}

pub fn shape_kind(body: Body) -> i32 {
    match body.shape {
        Shape::Sphere(_) => 0,
        Shape::Box => 1,
    }
}
"#;

#[test]
fn module_file_match_must_not_clone_scrutinee() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("match ") && rs.contains(".clone()");
    assert!(
        !bad,
        "RED match scrutinee cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
