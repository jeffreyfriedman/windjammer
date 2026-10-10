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

//! DB-422: Copy `f32` array index used in arithmetic must not `.clone()`.
//!
//! Product `frustum_culling.rs`:
//!   `view_proj[3].clone() + view_proj[0]`
//! WJ is `view_proj[3] + view_proj[0]`.
//! Distinct from DB-393 (`y.clone()` local i32), DB-346 (`color.r.clone()`),
//! and DB-355 (Copy Vec3).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn add_pair(view_proj: [f32; 16]) -> f32 {
    view_proj[3] + view_proj[0]
}
"#;

#[test]
fn module_file_copy_f32_index_arith_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "Copy f32 array index cloned in arithmetic:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
