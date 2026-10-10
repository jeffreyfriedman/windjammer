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

//! Regression: module file copy f32 formal field assign must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct FrameTimer {
    pub delta_time: f32,
    pub time_accumulator: f32,
}

impl FrameTimer {
    pub fn update(self, actual_dt: f32) {
        self.delta_time = actual_dt
        self.time_accumulator = self.time_accumulator + actual_dt
    }
}
"#;

#[test]
fn module_file_copy_f32_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("actual_dt.clone()");
    assert!(
        !bad,
        "RED: Copy f32 formal cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}
