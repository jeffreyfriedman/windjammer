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

//!
//!   `let a = tri_norm.a; all_corners.push(a.clone()); uvs.push(a.clone())`
//! WJ uses bare `a` / `b` / `c`.
//! into tuple lit for push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct UvCoord {
    pub u: f32,
    pub v: f32,
}

pub struct TriNorm {
    pub a: UvCoord,
    pub b: UvCoord,
    pub c: UvCoord,
}

pub fn collect_corners(tri_norm: TriNorm) -> Vec<UvCoord> {
    let a = tri_norm.a
    let b = tri_norm.b
    let c = tri_norm.c
    let mut all_corners: Vec<UvCoord> = Vec::new()
    all_corners.push(a)
    all_corners.push(b)
    all_corners.push(c)
    all_corners
}
"#;

#[test]
fn module_file_copy_struct_local_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("a.clone()") || rs.contains("b.clone()") || rs.contains("c.clone()");
    assert!(
        !bad,
        "RED: Copy struct local cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

