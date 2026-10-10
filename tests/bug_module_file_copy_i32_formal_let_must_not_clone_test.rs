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

//! Copy `i32` formal into local `let` must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn shrink_radius(radius: i32, dy: i32, height: i32) -> i32 {
    let mut r = radius
    if dy < 2 || dy > height - 3 {
        r = radius - 1
        if r < 1 {
            r = 1
        }
    }
    r
}
"#;

#[test]
fn module_file_copy_i32_formal_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("radius.clone()");
    assert!(
        !bad,
        "RED Copy i32 formal cloned into let:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
