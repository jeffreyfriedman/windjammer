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
//!   `get_affected_files(self.watches[i].clone().path.clone())`
//! Prefer `self.watches[i].path.clone()` once (or borrow if formal is `&str`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Watch {
    pub path: string,
}

pub struct DepGraph {
    pub n: i32,
}

pub fn get_affected_files(dep: DepGraph, path: string) -> Vec<string> {
    let mut out = Vec::new()
    out.push(path)
    out
}

pub struct Loader {
    pub watches: Vec<Watch>,
}

impl Loader {
    pub fn affected(self, dep: DepGraph, i: usize) -> Vec<string> {
        get_affected_files(dep, self.watches[i].path)
    }
}
"#;

#[test]
fn module_file_index_string_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().path.clone()") || rs.contains("].clone().path");
    assert!(
        !bad,
        "RED: indexed String path double-cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

