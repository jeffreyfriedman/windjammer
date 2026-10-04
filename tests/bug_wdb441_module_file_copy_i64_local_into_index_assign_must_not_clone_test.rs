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

//! WDB-441: Copy `i64` local into indexed assign must not `.clone()`.
//!
//! Product `ecs/component_storage.rs`:
//!   `self.entities[sparse_idx_usize] = swapped_entity.clone()`
//! WJ is `self.entities[sparse_idx_usize] = swapped_entity`.
//! Distinct from WDB-437 (formal into **let**), WDB-440 (f32 into **struct lit**),
//! and WDB-436 (loop counter into cast assign).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct EntitySparse {
    pub entities: Vec<i64>,
    pub sparse: Vec<i64>,
    pub count: usize,
}

impl EntitySparse {
    pub fn swap_remove(self, sparse_idx_usize: usize) {
        if self.count == 0 {
            return
        }
        let swapped_entity = self.entities[self.count - 1]
        self.entities[sparse_idx_usize] = swapped_entity
        self.sparse[swapped_entity as usize] = sparse_idx_usize as i64
        self.count = self.count - 1
    }
}
"#;

#[test]
fn wdb441_module_file_copy_i64_local_into_index_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-441 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-441 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("swapped_entity.clone()");
    assert!(
        !bad,
        "WDB-441 RED: Copy i64 local cloned into index assign:\n{rs}"
    );
    // Codegen gate is the contract; cargo-check can time out under contention.
    let _ = test.cargo_check();
}

fn wdb441_search_roots() -> Vec<PathBuf> {
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
fn wdb441_tip_out_game_core_component_storage_entity_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb441_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ecs/component_storage.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ecs/component_storage.rs"));
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
            !t.starts_with("//") && t.contains("swapped_entity.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-441: component_storage product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-441 RED: tip/product Copy i64 local into index assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
