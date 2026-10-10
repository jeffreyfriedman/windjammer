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

//! Regression: module file owned string must not receive ref clone lsqb.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// Format/`len`-only helpers demote to `&str` (P3.264). Force owned String formals
// via concat into a struct field — mirrors `filename: String, content: String`.
const SRC: &str = r#"
pub struct Loaded {
    pub filename: string,
    pub content: string,
}

pub fn load_vertex_csv_content(filename: string, content: string) -> Loaded {
    Loaded {
        filename: filename,
        content: content,
    }
}

pub fn load_csv_file(filename: string, content: string) -> Loaded {
    let a = load_vertex_csv_content(filename, content)
    let _b = load_vertex_csv_content(filename, content)
    a
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn load_vertex_csv_content(filename: String")
        || rs.contains("fn load_vertex_csv_content(mut filename: String");
    assert!(
        owned,
        "expected owned String formals on load_vertex_csv_content:\n{rs}"
    );
    let bad = rs.contains("&filename.clone()")
        || rs.contains("&content.clone()")
        || rs.contains("load_vertex_csv_content(&");
    assert!(
        !bad,
        "RED: owned load_vertex_csv_content received &owned.clone() / &args:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
