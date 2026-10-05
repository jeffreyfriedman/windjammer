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

//! WDB-451: Copy `i64` **formal** into field assign must not `.clone()`.
//!
//! Product `scripting/live_reload.rs` `poll`:
//!   `self.last_poll_time = current_time.clone()`
//! WJ uses bare `current_time`.
//! Distinct from WDB-393 (i32 formal field assign), WDB-448 (f32), WDB-449 (bool),
//! and WDB-441 (i64 local into **index** assign).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct LiveReloadSystem {
    pub last_poll_time: i64,
    pub poll_interval_ms: i32,
}

impl LiveReloadSystem {
    pub fn poll(self, current_time: i64) -> i32 {
        if current_time - self.last_poll_time < self.poll_interval_ms as i64 {
            return 0
        }
        self.last_poll_time = current_time
        1
    }
}
"#;

#[test]
fn wdb451_module_file_copy_i64_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-451 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-451 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("current_time.clone()");
    assert!(
        !bad,
        "WDB-451 RED: Copy i64 formal cloned into field assign:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb451_search_roots() -> Vec<PathBuf> {
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
fn wdb451_tip_out_game_core_live_reload_i64_formal_field_assign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb451_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/scripting/live_reload.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/live_reload.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/scripting/live_reload.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/live_reload.rs"));
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
            !t.starts_with("//") && t.contains("current_time.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-451: live_reload product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-451 RED: tip/product Copy i64 formal into field assign clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
