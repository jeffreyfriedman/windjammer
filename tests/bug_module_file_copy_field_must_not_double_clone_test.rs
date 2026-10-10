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

//! Regression: module file copy field must not double clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Coord {
    pub x: i32,
    pub z: i32,
}

pub struct Chunk {
    pub coord: Coord,
    pub name: string,
}

pub fn chebyshev(a: Coord, b: Coord) -> i32 {
    let dx = a.x - b.x
    if dx < 0 {
        -dx
    } else {
        dx
    }
}

pub struct World {
    pub chunks: Vec<Chunk>,
}

impl World {
    pub fn dist_from_chunk(self, i: usize, b: Coord) -> i32 {
        chebyshev(self.chunks[i].coord, b)
    }
}
"#;

#[test]
fn module_file_copy_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().coord") || rs.contains(".coord.clone()");
    assert!(
        !bad,
        "RED: indexed Copy field double-cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
