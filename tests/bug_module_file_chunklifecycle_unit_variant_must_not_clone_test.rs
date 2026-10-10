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
//! WJ source is `self.chunks[i].state = ChunkLifecycleState::Loading`.
//! Numbered 403 to stay past compiler 395–400 and DB 401/402.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum ChunkLifecycleState {
    Unloaded,
    Loading,
    Loaded,
}

pub struct Chunk {
    pub state: ChunkLifecycleState,
}

pub fn mark_loading() -> Chunk {
    Chunk {
        state: ChunkLifecycleState::Loading,
    }
}

pub fn mark_unloaded() -> Chunk {
    Chunk {
        state: ChunkLifecycleState::Unloaded,
    }
}
"#;

#[test]
fn module_file_chunklifecycle_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("ChunkLifecycleState::Loading.clone()")
        || rs.contains("ChunkLifecycleState::Unloaded.clone()")
        || rs.contains("ChunkLifecycleState::Loaded.clone()");
    assert!(
        !bad,
        "RED: Copy ChunkLifecycleState unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

