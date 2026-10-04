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

//! WDB-442: Copy `u32` local into `let` must not `.clone()`.
//!
//! Product `world/async_loader.rs`:
//!   `let mut step = budget.clone()`
//! WJ is `let mut step = budget`.
//! Distinct from WDB-437 (i32 **formal** into let), WDB-441 (i64 into **index assign**),
//! and WDB-440 (f32 into **struct lit**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn pump_budget(max_bytes: u32, remaining: u32) -> u32 {
    let mut budget = max_bytes
    let mut loaded: u32 = 0
    while budget > 0 {
        let mut step = budget
        if step > remaining {
            step = remaining
        }
        loaded = loaded + step
        budget = budget - step
        if remaining == 0 {
            break
        }
    }
    loaded
}
"#;

#[test]
fn wdb442_module_file_copy_u32_local_into_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-442 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-442 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("budget.clone()") || rs.contains("step.clone()");
    assert!(
        !bad,
        "WDB-442 RED: Copy u32 local cloned into let:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb442_search_roots() -> Vec<PathBuf> {
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
fn wdb442_tip_out_game_core_async_loader_budget_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb442_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/world/async_loader.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/world/async_loader.rs"));
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
            !t.starts_with("//") && t.contains("budget.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-442: async_loader product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-442 RED: tip/product Copy u32 local into let clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
