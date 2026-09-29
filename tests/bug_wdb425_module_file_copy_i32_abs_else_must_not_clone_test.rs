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

//! WDB-425: Copy `i32` loop var in abs else-branch must not `.clone()`.
//!
//! Product `component_viewer_controls.rs`:
//!   `if dx < 0 { -dx } else { dx.clone() }`
//! WJ is `if dx < 0 { -dx } else { dx }`.
//! Distinct from WDB-393 (`cursor_x = x.clone()` assignment) and WDB-422
//! (Copy f32 index arith).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn abs_sum(n: i32) -> i32 {
    let mut total = 0
    for dx in -n..(n + 1) {
        let adx = if dx < 0 { -dx } else { dx }
        total = total + adx
    }
    total
}
"#;

#[test]
fn wdb425_module_file_copy_i32_abs_else_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-425 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-425 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "WDB-425 RED: Copy i32 abs else-branch cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-425 cargo-check");
}

fn wdb425_search_roots() -> Vec<PathBuf> {
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
fn wdb425_tip_out_game_core_abs_else_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb425_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/component_viewer_controls.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/scene/component_viewer_controls.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/scene/component_viewer_controls.rs"),
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
            !t.starts_with("//")
                && t.contains(".clone()")
                && (t.contains("dx.clone()") || t.contains("dz.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-425: component_viewer_controls product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-425 RED: tip/product cloned Copy i32 in abs else in:\n  {}",
        bad_paths.join("\n  ")
    );
}
