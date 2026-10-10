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

//! Copy `f32` formal into `let` must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn scatter_region(start_x: f32, start_z: f32, end_x: f32, end_z: f32) -> i32 {
    let mut count: i32 = 0
    let mut z = start_z
    while z < end_z {
        let mut x = start_x
        while x < end_x {
            count = count + 1
            x = x + 1.0
        }
        z = z + 1.0
    }
    count
}
"#;

#[test]
fn module_file_copy_f32_formal_into_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("start_x.clone()") || rs.contains("start_z.clone()");
    assert!(
        !bad,
        "RED Copy f32 formal cloned into let:\n{rs}"
    );
    let _ = test.cargo_check();
}
