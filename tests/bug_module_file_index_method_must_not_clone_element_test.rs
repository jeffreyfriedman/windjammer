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

//! Regression: module file index method must not clone element.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Level {
    pub id: i32,
}

impl Level {
    pub fn mesh_id(self) -> i32 {
        self.id
    }
}

pub struct Lod {
    pub levels: Vec<Level>,
}

pub fn first_id(lod: Lod) -> i32 {
    if lod.levels.len() > 0 {
        lod.levels[0].mesh_id()
    } else {
        0
    }
}
"#;

#[test]
fn module_file_index_method_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone()") || rs.contains("].clone().");
    assert!(
        !bad,
        "RED: indexed method cloned element:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
