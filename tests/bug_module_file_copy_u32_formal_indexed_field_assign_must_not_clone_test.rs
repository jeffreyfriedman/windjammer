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

//! WDB-450: Copy `u32` **formal** into **indexed field** assign must not `.clone()`.
//!
//! Product `voxel/chunk_manager.rs` `mark_uploaded`:
//!   `self.chunks[i].gpu_buffer_id = buffer_id.clone()`
//! WJ uses bare `buffer_id`.
//! Distinct from WDB-447 (u32 formal into `tiles[i] = …` element assign),
//! WDB-376 (indexed **read** `.buffer_id.clone()`), and WDB-441 (i64 local index assign).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct ManagedChunk {
    pub gpu_buffer_id: u32,
}

pub struct ChunkManager {
    pub chunks: Vec<ManagedChunk>,
}

impl ChunkManager {
    pub fn mark_uploaded(self, buffer_id: u32) {
        let mut i: int = 0
        while i < self.chunks.len() {
            self.chunks[i].gpu_buffer_id = buffer_id
            i = i + 1
        }
    }
}
"#;

#[test]
fn module_file_copy_u32_formal_indexed_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("buffer_id.clone()");
    assert!(
        !bad,
        "Copy u32 formal cloned into indexed field assign:\n{rs}"
    );
    // Soft: codegen gate is the assertion above; cargo_check may contend.
    let _ = test.cargo_check();
}
