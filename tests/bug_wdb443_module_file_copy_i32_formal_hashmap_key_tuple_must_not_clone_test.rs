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

//! WDB-443: Copy `i32` formals into HashMap key tuple must not `.clone()`.
//!
//! Product `ai/astar_grid.rs`:
//!   `g_score.insert((start_x.clone(), start_y.clone()), 0.0_f32)`
//! WJ is `g_score.insert((start_x, start_y), 0.0)`.
//! Distinct from WDB-438 (tuple lit into **Vec::push**), WDB-437 (formal into **let**),
//! and WDB-441 (i64 into **index assign**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
use std::collections::HashMap

pub fn seed_g_score(start_x: i32, start_y: i32) -> HashMap<(i32, i32), f32> {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((start_x, start_y), 0.0)
    g_score
}

pub fn update_g_score(g_score: HashMap<(i32, i32), f32>, nx: i32, ny: i32, tentative_g: f32) {
    g_score.insert((nx, ny), tentative_g)
}
"#;

#[test]
fn wdb443_module_file_copy_i32_formal_hashmap_key_tuple_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-443 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-443 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("start_x.clone()")
        || rs.contains("start_y.clone()")
        || rs.contains("nx.clone()")
        || rs.contains("ny.clone()");
    assert!(
        !bad,
        "WDB-443 RED: Copy i32 formal cloned into HashMap key tuple:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb443_search_roots() -> Vec<PathBuf> {
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
fn wdb443_tip_out_game_core_astar_grid_hashmap_key_tuple_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb443_search_roots() {
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
                && t.contains("insert((")
                && (t.contains("start_x.clone()")
                    || t.contains("start_y.clone()")
                    || t.contains("nx.clone()")
                    || t.contains("ny.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-443: astar_grid product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-443 RED: tip/product Copy i32 HashMap key tuple clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
