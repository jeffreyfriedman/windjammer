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

//! WDB-448: Copy `f32` **formal** into field assign must not `.clone()`.
//!
//! Product `game_framework/game_loop.rs`:
//!   `self.delta_time = actual_dt.clone()`
//! WJ uses bare `actual_dt`.
//! Distinct from WDB-393 (i32 formal field assign), WDB-445 (f32 formal into let),
//! and WDB-440 (f32 into struct lit).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct FrameTimer {
    pub delta_time: f32,
    pub time_accumulator: f32,
}

impl FrameTimer {
    pub fn update(self, actual_dt: f32) {
        self.delta_time = actual_dt
        self.time_accumulator = self.time_accumulator + actual_dt
    }
}
"#;

#[test]
fn wdb448_module_file_copy_f32_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-448 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-448 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("actual_dt.clone()");
    assert!(
        !bad,
        "WDB-448 RED: Copy f32 formal cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb448_search_roots() -> Vec<PathBuf> {
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
fn wdb448_tip_out_game_core_game_loop_f32_formal_field_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb448_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/game_framework/game_loop.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/game_loop.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/game_framework/game_loop.rs"),
        );
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/game_loop.rs"));
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
            !t.starts_with("//") && t.contains("actual_dt.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-448: game_loop product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-448 RED: tip/product Copy f32 formal into field assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
