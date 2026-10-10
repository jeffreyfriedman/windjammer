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

//! indexed `&self` `get_id()` must not emit `].clone().get_id()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Cell {
    pub id: u32,
}

impl Cell {
    pub fn get_id(self) -> u32 {
        self.id
    }
}

pub struct Partition {
    pub cells: Vec<Cell>,
}

impl Partition {
    pub fn first_id(self, i: usize) -> u32 {
        self.cells[i].get_id()
    }
}
"#;

#[test]
fn module_file_index_get_id_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().get_id()");
    assert!(
        !bad,
        "RED indexed get_id cloned the element:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
