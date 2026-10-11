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

//! an owned `Vec` return passed into an owned formal must not `.clone()`.
//!
//! Product `editor/csg.rs` `csg_union` / `csg_subtract` / `csg_intersect`:
//!   `build_bsp_from_polygons(clone_polygons(&a).clone())`
//! WJ is `build_bsp_from_polygons(clone_polygons(a))`.
//! `clone_polygons` already returns `Vec<CsgPolygon>`.
//! Distinct from Copy-element `.clone()` gates (through ).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Poly {
    pub verts: Vec<int>,
}

fn clone_polys(polys: Vec<Poly>) -> Vec<Poly> {
    let mut out = Vec::new()
    for i in 0..polys.len() {
        let p = polys[i]
        out.push(p)
    }
    out
}

fn build_tree(polygons: Vec<Poly>) -> int {
    polygons.len()
}

pub fn combine(a: Vec<Poly>, b: Vec<Poly>) -> int {
    let mut ta = build_tree(clone_polys(a))
    let mut tb = build_tree(clone_polys(b))
    ta + tb
}
"#;

#[test]
fn module_file_owned_vec_return_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.lines().any(|line| line.contains("clone_polys(") && line.contains(".clone()"));
    assert!(
        !bad,
        "owned Vec return cloned at the call:\n{rs}"
    );
    let _ = test.cargo_check();
}
