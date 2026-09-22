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

//! P3.424: read-only method (`self.field` reads only) must emit `&self`, not owned `self`.
//! Product: voxel_gpu_passes `update_raymarch_params(self)` called from `&mut self` loop → E0507.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Renderer {
    pub n: i32,
    pub buf: i32,
}

impl Renderer {
    pub fn update_params(self) {
        let x = self.n
        let _ = x + self.buf
    }

    pub fn tick(self) {
        self.update_params()
        self.update_params()
    }
}
"#;

#[test]
fn readonly_method_must_emit_shared_self() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.424 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.424 MultiFile lib.rs:\n{rs}");
    assert!(
        rs.contains("fn update_params(&self)") || rs.contains("fn update_params(&mut self)"),
        "P3.424 RED: update_params must not take owned self:\n{rs}"
    );
    assert!(
        !rs.contains("fn update_params(self)"),
        "P3.424 RED: owned self on read-only method:\n{rs}"
    );
    test.cargo_check().expect("P3.424 cargo-check");
}
