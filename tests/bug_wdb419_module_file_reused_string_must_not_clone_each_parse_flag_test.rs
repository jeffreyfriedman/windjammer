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

//! WDB-419: reused owned `string` formal into several owned helpers must not
//! `parse_flag(line.clone(), …)` at every callsite.
//!
//! Product `agent_playtest_protocol.rs`:
//!   `parse_flag(line.clone(), "forward")` then `parse_flag(line.clone(), "back")` …
//! WJ is `parse_flag(line, "forward")` / `parse_flag(line, "back")`.
//! Distinct from WDB-413 (`string_len(line.clone())` in one comparison) and
//! WDB-409 (stored `set(name)` must stay owned).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn parse_flag(line: string, key: string) -> bool {
    if line.len() == 0 {
        false
    } else {
        key.len() > 0
    }
}

pub fn from_line(line: string) -> i32 {
    let f = parse_flag(line, "forward")
    let b = parse_flag(line, "back")
    if f {
        1
    } else if b {
        2
    } else {
        0
    }
}
"#;

#[test]
fn wdb419_module_file_reused_string_must_not_clone_each_parse_flag() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-419 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-419 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("line.clone()");
    assert!(
        !cloned,
        "WDB-419 RED: reused string cloned at each parse_flag callsite:\n{rs}"
    );
    test.cargo_check().expect("WDB-419 cargo-check");
}

fn wdb419_search_roots() -> Vec<PathBuf> {
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
fn wdb419_tip_out_game_core_parse_flag_must_not_clone_line() {
    let mut paths = Vec::new();
    for dir in wdb419_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/testing/agent_playtest_protocol.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/testing/agent_playtest_protocol.rs"),
        );
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
            !t.starts_with("//") && t.contains("parse_flag(line.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-419: agent_playtest_protocol product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-419 RED: tip/product parse_flag cloned line in:\n  {}",
        bad_paths.join("\n  ")
    );
}
