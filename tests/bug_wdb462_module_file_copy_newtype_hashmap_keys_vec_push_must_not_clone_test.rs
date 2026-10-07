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

//! WDB-462: Copy **newtype** from `HashMap.keys()` into `Vec::push` must not `.clone()`.
//!
//! Product `state_machine/machine.rs` `all_state_ids`:
//!   `for id in self.states.keys() { result.push(id.clone()) }`
//! WJ uses bare `result.push(id)` (`StateId` is `u32` newtype, Copy).
//! Distinct from WDB-457 (owned Copy newtype **local** into push), WDB-459
//! (Copy **struct** local into push), WDB-431 (Copy u64 **field** into push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct StateId {
    pub value: u32,
}

impl StateId {
    pub fn new(value: u32) -> StateId {
        StateId { value: value }
    }

    pub fn value(self) -> u32 {
        self.value
    }
}

pub struct StateMachine {
    pub states: Map<StateId, i32>,
}

impl StateMachine {
    pub fn new() -> StateMachine {
        StateMachine { states: Map::new() }
    }

    pub fn all_state_ids(self) -> Vec<StateId> {
        let mut result: Vec<StateId> = Vec::new()
        for id in self.states.keys() {
            result.push(id)
        }
        result
    }
}
"#;

#[test]
fn wdb462_module_file_copy_newtype_hashmap_keys_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-462 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-462 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("id.clone()");
    assert!(
        !bad,
        "WDB-462 RED: Copy newtype from HashMap.keys() cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb462_search_roots() -> Vec<PathBuf> {
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
fn wdb462_tip_out_game_core_state_machine_keys_vec_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb462_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/state_machine/machine.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/state_machine/machine.rs"));
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
            !t.starts_with("//") && t.contains("result.push(id.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-462: state_machine/machine product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-462 RED: tip/product Copy newtype HashMap.keys() into Vec::push clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
