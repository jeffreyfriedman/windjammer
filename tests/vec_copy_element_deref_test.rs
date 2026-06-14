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
))]

//! TDD: Vec<Copy> indexing must compile in Rust without E0614.
//!
//! For `T: Copy`, Rust's index desugaring already produces `T` in value position. Explicit
//! `*(vec[i])` is invalid (E0614: type `T` cannot be dereferenced) for both `Vec<T>` and `&Vec<T>`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn test_vec_element_copy_type_auto_deref() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "lib.wj",
        r#"
pub fn process(values: Vec<f32>) -> f32 {
    let first = values[0]
    first + 1.0
}
"#,
    );

    let map = test.compile().expect("Windjammer multipass compile");
    let lib_rs = map.get("lib.rs").expect("lib.rs present");
    assert!(
        !lib_rs.contains("*(values[") && !lib_rs.contains("* (values["),
        "must NOT emit explicit deref for Vec<Copy> index (E0614); got:\n{lib_rs}"
    );

    test.assert_compiles_without_error();
}

#[test]
fn test_vec_element_copy_i32_auto_deref() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "lib.wj",
        r#"
pub fn sum_first_two(xs: Vec<i32>) -> i32 {
    xs[0] + xs[1]
}
"#,
    );

    let map = test.compile().expect("compile");
    let lib_rs = map.get("lib.rs").expect("lib.rs");
    assert!(
        !lib_rs.contains("*(xs[") && !lib_rs.contains("* (xs["),
        "must NOT emit explicit deref for Vec<i32> indices; got:\n{lib_rs}"
    );
    test.assert_compiles_without_error();
}
