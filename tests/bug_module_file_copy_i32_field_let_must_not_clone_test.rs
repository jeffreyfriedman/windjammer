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

//!
//!   `let size = chunk.size.clone();`
//! WJ is `let size = chunk.size`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Chunk {
    pub size: i32,
}

pub fn extent(chunk: Chunk) -> i32 {
    let size = chunk.size
    size * size
}
"#;

#[test]
fn module_file_copy_i32_field_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "RED: Copy i32 field cloned into local:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

