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

//! WDB-478: a `for` binding moved into `Vec::push` on exclusive `if let` arms
//! must not `.clone()`, including `.clone().clone()` on the nested arms.
//!
//! Product `dialogue_system.rs` `filter_choices`:
//!   `available.push(choice.clone().clone())` and `available.push(choice.clone())`
//! WJ is `available.push(choice)` once per arm. `choice` comes from
//! `for choice in choices` and is not used again after the push.
//! Distinct from WDB-474 (one last-use local, no exclusive arms) and WDB-471
//! (a reused `Vec` argument `.clone().clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Choice {
    pub id: int,
}

pub struct World {
    pub flag: int,
}

pub struct Player {
    pub level: int,
}

pub fn filter_choices(choices: Vec<Choice>, world: Option<World>, player: Option<Player>) -> Vec<Choice> {
    let mut available = Vec::new()
    for choice in choices {
        if let Some(w) = world {
            if let Some(p) = player {
                if w.flag > 0 && p.level > 0 {
                    available.push(choice)
                }
            } else {
                available.push(choice)
            }
        } else {
            available.push(choice)
        }
    }
    available
}
"#;

#[test]
fn wdb478_module_file_exclusive_arm_loop_binding_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-478 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-478 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("choice.clone()"),
        "WDB-478 RED: exclusive-arm loop binding cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb478_search_roots() -> Vec<PathBuf> {
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
fn wdb478_tip_out_dialogue_choice_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb478_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/dialogue_system.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/dialogue_system.rs"));
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
            !t.starts_with("//") && t.contains("push(choice.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-478: dialogue product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-478 RED: tip/product exclusive-arm loop binding cloned into push:\n  {}",
        bad_paths.join("\n  ")
    );
}
