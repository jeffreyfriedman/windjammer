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

//! its fields, must not `.clone().clone()`. One clone on the push-only arm
//! is enough.
//!
//!   `back.push(poly.clone().clone())`
//!   while the spanning else reads `poly.vertices`
//! WJ is `back.push(poly)` on the `polygon_type == 2` arm.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Poly {
    pub kind: int,
    pub verts: Vec<int>,
}

pub fn split(poly: Poly, kind: int) -> int {
    let mut front = Vec::new()
    let mut back = Vec::new()
    if kind == 1 {
        front.push(poly)
    } else {
        if kind == 2 {
            back.push(poly)
        } else {
            let n = poly.verts.len()
            front.push(poly)
            let _kept = n
        }
    }
    front.len() + back.len()
}
"#;

#[test]
fn module_file_nested_if_push_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "RED: nested-if push double-cloned the struct:\n{rs}"
    );
    let _ = test.cargo_check();
}

