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

//! indexed Copy **tuple** into `Vec::push` must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn reverse_path(path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut rev: Vec<(i32, i32)> = Vec::new()
    let mut k: int = path.len() - 1
    while k >= 0 {
        rev.push(path[k])
        k = k - 1
    }
    rev
}
"#;

#[test]
fn module_file_indexed_copy_tuple_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "RED indexed Copy tuple cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}
