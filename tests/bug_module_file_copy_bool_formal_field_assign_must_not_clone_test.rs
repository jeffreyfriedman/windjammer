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

//! Copy `bool` **formal** into field assign must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Slider {
    pub enabled: bool,
    pub is_dragging: bool,
}

impl Slider {
    pub fn set_enabled(self, enabled: bool) {
        self.enabled = enabled
        if !enabled {
            self.is_dragging = false
        }
    }
}
"#;

#[test]
fn module_file_copy_bool_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("enabled.clone()");
    assert!(
        !bad,
        "RED Copy bool formal cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}
