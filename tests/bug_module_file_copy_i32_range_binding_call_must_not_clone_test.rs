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

//! Regression: module file copy i32 range binding call must not clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
fn paint(x: i32, y: i32, z: i32) -> i32 {
    x + y + z
}

pub fn carve(x0: i32, z0: i32, x1: i32, z1: i32, floor_y: i32, ceil_y: i32) -> i32 {
    let mut n = 0
    for x in x0 + 1..x1 {
        for z in z0 + 1..z1 {
            for y in floor_y + 1..ceil_y {
                n = n + paint(x, y, z)
            }
        }
    }
    n
}
"#;

#[test]
fn module_file_copy_i32_range_binding_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x.clone()") || rs.contains("y.clone()") || rs.contains("z.clone()");
    assert!(
        !bad,
        "RED: Copy i32 range binding cloned at the call:\n{rs}"
    );
    let _ = test.cargo_check();
}
