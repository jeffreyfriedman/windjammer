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

//! WDB-429: Copy match-arm payloads (`i32`/`f32`/`bool`) must not `.clone()`.
//!
//! Product `state_machine/state.rs`:
//!   `Some(StateDataValue::Int(value)) => Some(value.clone())`
//!   (same for Float/Bool)
//! WJ is `Some(value)` with no `.clone()`.
//! Distinct from WDB-393 (formal field assign), WDB-367/380 (`None.clone()`),
//! WDB-416 (`a.clone().as_float()`), and WDB-417 (owned struct match arm).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Cell {
    Int(i32),
    Float(f32),
    Flag(bool),
    Empty,
}

pub fn get_int(c: Cell) -> Option<i32> {
    match c {
        Cell::Int(value) => Some(value),
        _ => None,
    }
}

pub fn get_float(c: Cell) -> Option<f32> {
    match c {
        Cell::Float(value) => Some(value),
        _ => None,
    }
}

pub fn get_flag(c: Cell) -> Option<bool> {
    match c {
        Cell::Flag(value) => Some(value),
        _ => None,
    }
}
"#;

#[test]
fn wdb429_module_file_copy_match_arm_payload_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-429 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-429 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("value.clone()");
    assert!(
        !bad,
        "WDB-429 RED: Copy match-arm payload cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-429 cargo-check");
}

fn wdb429_search_roots() -> Vec<PathBuf> {
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
fn wdb429_tip_out_game_core_statedata_copy_payload_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb429_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/state_machine/state.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/state_machine/state.rs"));
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
                && (t.contains("StateDataValue::Int(value)) => Some(value.clone())")
                    || t.contains("StateDataValue::Float(value)) => Some(value.clone())")
                    || t.contains("StateDataValue::Bool(value)) => Some(value.clone())"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-429: state_machine/state product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-429 RED: tip/product cloned Copy StateDataValue payload:\n  {}",
        bad_paths.join("\n  ")
    );
}
