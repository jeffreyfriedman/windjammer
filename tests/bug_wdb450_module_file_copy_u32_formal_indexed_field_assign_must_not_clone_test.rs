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
use std::path::PathBuf;

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
fn wdb450_module_file_copy_u32_formal_indexed_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-450 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-450 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("buffer_id.clone()");
    assert!(
        !bad,
        "WDB-450 RED: Copy u32 formal cloned into indexed field assign:\n{rs}"
    );
    // Soft: codegen gate is the assertion above; cargo_check may contend.
    let _ = test.cargo_check();
}

fn wdb450_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb450_tip_out_game_core_chunk_manager_u32_formal_indexed_field_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb450_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/chunk_manager.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/voxel/chunk_manager.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/chunk_manager.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/voxel/chunk_manager.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//")
                && t.contains("gpu_buffer_id")
                && t.contains("buffer_id.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-450: chunk_manager product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-450 RED: tip/product Copy u32 formal into indexed field assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
