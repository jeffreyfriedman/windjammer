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

//! WDB-336: `&mut Vec` formal must not receive `&mut data.clone()` (temp).
//!
//! Tip output for a game project `rendering/mesh_renderer.rs`:
//!   `push_mat4(&mut data.clone(), …)` / `push_mat4(&mut mat4_floats.clone(), …)`
//!   with `data: &mut Vec<f32>` → E0716 / discarded mutation.
//! Source WJ: mutate the real buffer (`push_mat4(data, …)` → `&mut data`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Mat4 {
    pub m: f32,
}

pub fn push_mat4(data: Vec<f32>, m: Mat4) {
    data.push(m.m)
}

// Product shape (mesh_renderer): local `let mut data` reused after demoted `&mut Vec`
// must emit `&mut data`, never `&mut data.clone()`.
pub fn upload(view: Mat4, proj: Mat4) -> Vec<f32> {
    let mut data: Vec<f32> = Vec::new()
    push_mat4(data, view)
    push_mat4(data, proj)
    data
}
"#;

#[test]
fn module_file_mut_vec_must_not_borrow_clone_temp() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad_temp = rs.contains("&mut data.clone()")
        || rs.contains("&mut ") && rs.contains(".clone()");
    assert!(
        !bad_temp,
        "mut Vec received &mut <temp>.clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
