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

//! WDB-439: Copy `i32` from indexed tuple field `.0` / `.1` must not `.clone()`.
//!
//! Product `ai/astar_grid.rs`:
//!   `return self.nodes[index as usize].0.clone()`
//! WJ is `return self.nodes[index as usize].0`.
//! Distinct from WDB-363 (`.clone().field`), WDB-423 (destructure index),
//! and WDB-438 (tuple lit into `push`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct NodeStore {
    pub nodes: Vec<(i32, i32)>,
}

impl NodeStore {
    pub fn x_at(self, index: i32) -> i32 {
        if index < 0 {
            return 0
        }
        if (index as usize) >= self.nodes.len() {
            return 0
        }
        return self.nodes[index as usize].0
    }

    pub fn y_at(self, index: i32) -> i32 {
        if index < 0 {
            return 0
        }
        if (index as usize) >= self.nodes.len() {
            return 0
        }
        return self.nodes[index as usize].1
    }
}
"#;

#[test]
fn wdb439_module_file_copy_i32_index_tuple_field_return_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-439 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-439 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".0.clone()") || rs.contains(".1.clone()");
    assert!(
        !bad,
        "WDB-439 RED: Copy i32 indexed tuple field cloned on return:\n{rs}"
    );
    test.cargo_check().expect("WDB-439 cargo-check");
}

fn wdb439_search_roots() -> Vec<PathBuf> {
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
fn wdb439_tip_out_game_core_astar_grid_index_tuple_field_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb439_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ai/astar_grid.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/astar_grid.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ai/astar_grid.rs"));
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
                && (t.contains(".0.clone()") || t.contains(".1.clone()"))
                && t.contains("nodes[")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-439: astar_grid product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-439 RED: tip/product Copy i32 indexed tuple field clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
