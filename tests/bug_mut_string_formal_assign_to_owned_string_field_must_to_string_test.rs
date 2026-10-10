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

//! P3.418: `self.search_query = query` when tip demotes `query: string` to `&mut String`
//! must emit `.to_string()` (or `.clone()`) into owned `String` field — console/hierarchy.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Console {
    pub search_query: string,
}

impl Console {
    pub fn set_search_query(query: string) {
        self.search_query = query
    }
}
"#;

#[test]
fn mut_string_formal_assign_to_owned_string_field_must_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    assert!(
        rs.contains("query: &mut String")
            || rs.contains("query: &String")
            || rs.contains("query: &str"),
        "expected demoted query formal:\n{rs}"
    );
    assert!(
        !rs.contains("search_query = query;"),
        "bare assign of demoted query into owned String:\n{rs}"
    );
    assert!(
        rs.contains("search_query = query.to_string()")
            || rs.contains("search_query = query.clone()"),
        "expected to_string/clone into owned field:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
