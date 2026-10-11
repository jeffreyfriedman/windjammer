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

//! Copy `self` must not emit `self.clone().method()` into owned-self callee.
//!
//! Tip output for a game project `math/mat4.rs` (Mat4 is Copy):
//!   `self.clone().multiply(other)` / `self.clone().transpose().to_array()`
//! Prefer bare `self.multiply(other)` (Copy) — no `.clone()`.
//! Broader product cluster also emits `self.clone().…` on non-Copy types for `&self`
//! wrappers; this gate focuses on Copy Mat4.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Mat4 {
    pub m00: f32,
}

impl Mat4 {
    pub fn multiply(self, other: Mat4) -> Mat4 {
        Mat4 { m00: self.m00 * other.m00 }
    }

    pub fn mul(self, other: Mat4) -> Mat4 {
        self.multiply(other)
    }
}
"#;

#[test]
fn module_file_copy_self_must_not_clone_before_owned_method() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "Copy Mat4 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
