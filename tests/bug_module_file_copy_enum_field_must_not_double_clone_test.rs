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

//! Regression: module file copy enum field must not double clone.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum State {
    Unloaded,
    Loaded,
}

pub struct Chunk {
    pub state: State,
    pub name: string,
}

pub fn is_unloaded(s: State) -> bool {
    match s {
        State::Unloaded => true,
        State::Loaded => false,
    }
}

pub struct World {
    pub chunks: Vec<Chunk>,
}

impl World {
    pub fn first_unloaded(self, i: usize) -> bool {
        is_unloaded(self.chunks[i].state)
    }
}
"#;

/// Shape: match indexed Copy enum + arm mutates self.
const SRC_MATCH: &str = r#"
pub enum ChunkLifecycleState {
    Unloaded,
    Loading,
    Loaded,
}

pub struct Chunk {
    pub state: ChunkLifecycleState,
    pub name: string,
}

pub struct World {
    pub chunks: Vec<Chunk>,
}

impl World {
    pub fn tick_unload(self, i: usize) {
        match self.chunks[i].state {
            ChunkLifecycleState::Loaded => {
                self.chunks[i].state = ChunkLifecycleState::Unloaded
            },
            _ => {},
        }
    }
}
"#;

#[test]
fn module_file_copy_enum_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().state") || rs.contains(".state.clone()");
    assert!(
        !bad,
        "RED: indexed Copy enum field cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_match_indexed_copy_enum_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_MATCH);
    let map = test.compile().expect("match compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("match lib.rs:\n{rs}");
    let bad = rs.contains(".state.clone()") || rs.contains("].clone().state");
    assert!(
        !bad,
        "RED: match indexed Copy enum cloned:\n{rs}"
    );
    test.cargo_check().expect("match cargo-check");
}
