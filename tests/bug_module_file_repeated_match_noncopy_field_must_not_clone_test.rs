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

//! Regression: module file repeated match noncopy field must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Shape {
    Box(f32),
    Hull(Vec<f32>),
}

pub struct Body {
    pub shape: Shape,
}

pub fn kind_and_x(body: Body) -> f32 {
    let kind = match body.shape {
        Shape::Box(_) => 0.0,
        Shape::Hull(_) => 1.0,
    }
    let x = match body.shape {
        Shape::Box(v) => v,
        Shape::Hull(pts) => pts[0],
    }
    kind + x
}
"#;

#[test]
fn module_file_repeated_match_noncopy_field_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("shape.clone()");
    assert!(
        !cloned,
        "RED: repeated match cloned non-Copy field:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
