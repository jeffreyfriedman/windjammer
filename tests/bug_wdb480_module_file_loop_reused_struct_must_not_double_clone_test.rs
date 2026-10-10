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

//! WDB-480: a non-Copy value sent on every loop iteration must not
//! `.clone().clone()`. One clone per call is enough.
//!
//! Product `ai/squad_tactics.rs` `broadcast_to_nearby`:
//!   `squad.send_message(message.clone().clone())`
//! WJ is `squad.send_message(message)` inside `for squad in self.squads`.
//! `message` is reused, so a single clone per iteration is the move.
//! Distinct from WDB-471 (a reused `Vec` argument), WDB-479 (a `string` map
//! key), and P3.590b (`load_batch` may `name.clone()` once).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Message {
    pub text: string,
}

pub struct Squad {
    pub id: string,
}

impl Squad {
    pub fn send_message(self, message: Message) {
        let _kept = message.text
    }
}

pub struct Manager {
    pub squads: Vec<Squad>,
}

impl Manager {
    pub fn broadcast(self, message: Message) {
        for squad in self.squads {
            squad.send_message(message)
        }
    }
}
"#;

#[test]
fn wdb480_module_file_loop_reused_struct_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-480 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-480 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "WDB-480 RED: loop-reused struct double-cloned into send:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb480_search_roots() -> Vec<PathBuf> {
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
fn wdb480_tip_out_squad_message_must_not_double_clone() {
    let mut paths = Vec::new();
    for dir in wdb480_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ai/squad_tactics.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ai/squad_tactics.rs"));
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
            !t.starts_with("//") && t.contains("message.clone().clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-480: squad tactics product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-480 RED: tip/product loop-reused message double-cloned:\n  {}",
        bad_paths.join("\n  ")
    );
}
