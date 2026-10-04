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

//! WDB-436: `usize` loop counter assigned to `i32` with cast must not `.clone()`.
//!
//! Product `animation/clip.rs`:
//!   `before_idx = i.clone() as i32`
//! WJ is `before_idx = i`.
//! Distinct from WDB-394 (`best_idx = i` assign, no cast), WDB-432 (indexed
//! element `.clone() as`), and WDB-433 (struct field `.clone() as`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn pick_before(times: Vec<f32>, time: f32) -> i32 {
    let mut before_idx: i32 = 0
    let mut i: usize = 0
    while i < times.len() {
        if times[i] <= time {
            before_idx = i
            break
        }
        i = i + 1
    }
    before_idx
}
"#;

#[test]
fn wdb436_module_file_usize_loop_cast_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-436 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-436 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("i.clone()") || rs.contains(".clone() as i32");
    assert!(
        !bad,
        "WDB-436 RED: usize loop counter cloned on cast-assign:\n{rs}"
    );
    test.cargo_check().expect("WDB-436 cargo-check");
}

fn wdb436_search_roots() -> Vec<PathBuf> {
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
fn wdb436_tip_out_game_core_clip_loop_cast_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb436_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/animation/clip.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/animation/clip.rs"));
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
            !t.starts_with("//") && t.contains("before_idx") && t.contains("i.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-436: animation/clip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-436 RED: tip/product loop counter cast-assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
