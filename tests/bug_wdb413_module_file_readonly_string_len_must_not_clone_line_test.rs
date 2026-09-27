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

//! WDB-413: read-only `string_len(line)` after `&line` must not emit `line.clone()`.
//!
//! Product `agent_playtest_protocol.rs`:
//!   `key_value_start(&line, key) < gpu::string_len(line.clone())`
//! WJ is `key_value_start(line, key) < gpu::string_len(line)` — no `.clone()`.
//! Distinct from WDB-412 (Vec field clone), WDB-106 (explicit clone must stay),
//! and WDB-143 (reuse after owned consume must clone to compile).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
fn slen(s: string) -> u32 {
    if s == "" {
        0
    } else {
        1
    }
}

fn start(line: string, key: string) -> u32 {
    slen(key)
}

pub fn present(line: string, key: string) -> bool {
    start(line, key) < slen(line)
}
"#;

#[test]
fn wdb413_module_file_readonly_string_len_must_not_clone_line() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-413 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-413 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("line.clone()") || rs.contains("slen(line.clone()");
    assert!(
        !cloned,
        "WDB-413 RED: read-only string_len reused line via clone:\n{rs}"
    );
    test.cargo_check().expect("WDB-413 cargo-check");
}

fn wdb413_search_roots() -> Vec<PathBuf> {
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
fn wdb413_tip_out_game_core_readonly_string_len_must_not_clone_line() {
    let mut paths = Vec::new();
    for dir in wdb413_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/testing/agent_playtest_protocol.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/testing/agent_playtest_protocol.rs"));
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
            !t.starts_with("//") && t.contains("string_len(line.clone())")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-413: agent_playtest_protocol product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-413 RED: tip/product string_len cloned line in:\n  {}",
        bad_paths.join("\n  ")
    );
}
