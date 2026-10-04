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

//! WDB-438: Copy `i32` formal into tuple literal must not `.clone()` when
//! the same formal is also passed to a sibling call in that tuple.
//!
//! Product `ai/astar_grid.rs` `get_neighbors`:
//!   `result.push((x.clone(), y + 1_i32, self.get_cost(x, y + 1_i32)))`
//! WJ is `result.push((x, y + 1, self.get_cost(x, y + 1)))`.
//! Distinct from WDB-437 (formal into **let**), WDB-434 (const into **call**),
//! and WDB-423 (indexed tuple element).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct CostGrid {
    pub base: f32,
}

impl CostGrid {
    pub fn get_cost(self, x: i32, y: i32) -> f32 {
        self.base + (x as f32) * 0.01 + (y as f32) * 0.01
    }

    pub fn get_neighbors(self, x: i32, y: i32) -> Vec<(i32, i32, f32)> {
        let mut result: Vec<(i32, i32, f32)> = Vec::new()
        result.push((x, y + 1, self.get_cost(x, y + 1)))
        result.push((x + 1, y, self.get_cost(x + 1, y)))
        result.push((x, y - 1, self.get_cost(x, y - 1)))
        result.push((x - 1, y, self.get_cost(x - 1, y)))
        result
    }
}
"#;

#[test]
fn wdb438_module_file_copy_i32_formal_tuple_lit_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-438 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-438 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x.clone()") || rs.contains("y.clone()");
    assert!(
        !bad,
        "WDB-438 RED: Copy i32 formal cloned into tuple lit:\n{rs}"
    );
    test.cargo_check().expect("WDB-438 cargo-check");
}

fn wdb438_search_roots() -> Vec<PathBuf> {
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
fn wdb438_tip_out_game_core_astar_grid_tuple_lit_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb438_search_roots() {
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
                && t.contains("push((")
                && (t.contains("x.clone()") || t.contains("y.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-438: astar_grid product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-438 RED: tip/product Copy i32 formal tuple lit clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
