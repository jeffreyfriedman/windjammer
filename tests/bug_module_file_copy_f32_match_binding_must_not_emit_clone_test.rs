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

//! Copy `f32` match bindings must not emit `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum Shape {
    Sphere(f32),
    Capsule(f32, f32),
}

pub struct Body {
    pub shape: Shape,
    pub name: string,
}

pub fn dims(shape: Shape) -> (f32, f32, f32) {
    match shape {
        Shape::Sphere(r) => (r, 0.0, 0.0),
        Shape::Capsule(r, h) => (r, h, 0.0),
    }
}

pub fn dims_twice(body: Body) -> (f32, f32, f32) {
    let _ = match body.shape {
        Shape::Sphere(_) => 1,
        Shape::Capsule(_, _) => 2,
    }
    match body.shape {
        Shape::Sphere(r) => (r, 0.0, 0.0),
        Shape::Capsule(r, h) => (r, h, 0.0),
    }
}

/// `let (x,y,z) = match body.shape` on a borrowed `body` (jolt add_body).
pub fn dims_let_borrowed(body: Body) -> (f32, f32, f32) {
    let _kind = match body.shape {
        Shape::Sphere(_) => 1,
        Shape::Capsule(_, _) => 2,
    }
    let dims = match body.shape {
        Shape::Sphere(r) => (r, 0.0, 0.0),
        Shape::Capsule(r, h) => (r, h, 0.0),
    }
    dims
}
"#;

#[test]
fn module_file_copy_f32_match_binding_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // Scrutinee `.clone()` on non-Copy `body.shape` is required; only binding clones are RED.
    let bad = rs.lines().any(|line| {
        (line.contains("r.clone()") || line.contains("h.clone()"))
            && !line.trim_start().starts_with("//")
    });
    assert!(
        !bad,
        "RED Copy f32 match binding emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
