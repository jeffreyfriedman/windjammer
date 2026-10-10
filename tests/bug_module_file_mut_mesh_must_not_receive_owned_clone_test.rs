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

//! Regression: module file mut mesh must not receive owned clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Mesh {
    pub n: i32,
}

pub fn push_quad(mesh: Mesh, v: i32) {
    mesh.n = mesh.n + v
}

pub fn cube(mesh: Mesh) {
    push_quad(mesh, 1)
    push_quad(mesh, 2)
}
"#;

#[test]
fn module_file_mut_mesh_must_not_receive_owned_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // Fail if call site passes mesh.clone() (owned) — whether or not formal is &mut.
    let bad = rs.contains("mesh.clone()")
        || rs.contains("push_quad(mesh.clone()");
    assert!(
        !bad,
        "RED: mut mesh received owned mesh.clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
