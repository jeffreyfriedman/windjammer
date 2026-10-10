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

//! DB-446: Copy `i32` **local** into typed `let` must not `.clone()`.
//!
//! Product `voxel/meshing.rs`:
//!   `let mut slice_count: i32 = gd.clone()`
//!   `let mut x: i32 = u.clone()`
//! WJ uses bare `gd` / `u`.
//! Distinct from DB-437 (i32 **formal** into let), DB-442 (u32 **local** into let),
//! and DB-445 (f32 **formal** into let).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn greedy_axis(gw: i32, gh: i32, gd: i32, axis: i32) -> i32 {
    let mut slice_count: i32 = gd
    if axis == 0 {
        slice_count = gw
    }
    if axis == 1 {
        slice_count = gh
    }
    let mut u: i32 = 0
    let mut total: i32 = 0
    while u < slice_count {
        let mut x: i32 = u
        let mut y: i32 = u
        total = total + x + y
        u = u + 1
    }
    total
}
"#;

#[test]
fn module_file_copy_i32_local_typed_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("gd.clone()")
        || rs.contains("gw.clone()")
        || rs.contains("gh.clone()")
        || rs.contains("u.clone()");
    assert!(
        !bad,
        "Copy i32 local cloned into typed let:\n{rs}"
    );
    let _ = test.cargo_check();
}
