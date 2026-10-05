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

//! WDB-452: Copy `f32` **formal** into **local reassignment** must not `.clone()`.
//!
//! Product `ui/layout.rs` `layout_row`:
//!   `current_x = content_x.clone()`
//! WJ uses bare `content_x`.
//! Distinct from WDB-445 (f32 formal into **let**), WDB-448 (f32 formal into **field**),
//! and WDB-393 (i32 formal assign).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn layout_row(content_x: f32, content_width: f32, total_width: f32, align: i32) -> f32 {
    let mut current_x = 0.0
    if align == 0 {
        current_x = content_x
    }
    if align == 1 {
        current_x = content_x + (content_width - total_width) / 2.0
    }
    if align == 2 {
        current_x = content_x + (content_width - total_width)
    }
    if align == 3 {
        current_x = content_x
    }
    current_x
}
"#;

#[test]
fn wdb452_module_file_copy_f32_formal_local_reassign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-452 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-452 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("content_x.clone()");
    assert!(
        !bad,
        "WDB-452 RED: Copy f32 formal cloned into local reassignment:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb452_search_roots() -> Vec<PathBuf> {
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
fn wdb452_tip_out_game_core_layout_f32_formal_local_reassign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb452_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/ui/layout.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/layout.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/ui/layout.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/layout.rs"));
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
            !t.starts_with("//") && t.contains("content_x.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-452: layout product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-452 RED: tip/product Copy f32 formal into local reassignment clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
