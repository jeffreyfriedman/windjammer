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

//! indexed field access must not clone the whole element first.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Coord {
    pub x: i32,
}

pub struct Chunk {
    pub coord: Coord,
    pub dirty: bool,
}

pub struct Manager {
    pub chunks: Vec<Chunk>,
}

pub fn dirty_coords(m: Manager) -> Vec<Coord> {
    let mut result: Vec<Coord> = Vec::new()
    let mut i: usize = 0
    while i < m.chunks.len() {
        if m.chunks[i].dirty {
            result.push(m.chunks[i].coord)
        }
        i = i + 1
    }
    result
}
"#;

#[test]
fn module_file_index_field_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone()") || rs.contains("].clone().");
    assert!(
        !bad,
        "RED indexed access cloned whole element:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
