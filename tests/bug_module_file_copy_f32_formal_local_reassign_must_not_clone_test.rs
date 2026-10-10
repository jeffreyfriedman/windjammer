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

//! Regression: module file copy f32 formal local reassign must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn layout_row(content_x: f32, content_width: f32, total_width: f32, align: i32) -> f32 {
    let mut current_x = 0.0
    if align == 0 {
        current_x = content_x
    }
    if align == 1 {
        current_x = content_x + (content_width - total_width) / 2.0
    }
    if align == 2 {
        current_x = content_x + (content_width - total_width)
    }
    if align == 3 {
        current_x = content_x
    }
    current_x
}
"#;

#[test]
fn module_file_copy_f32_formal_local_reassign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("content_x.clone()");
    assert!(
        !bad,
        "RED: Copy f32 formal cloned into local reassignment:\n{rs}"
    );
    let _ = test.cargo_check();
}
