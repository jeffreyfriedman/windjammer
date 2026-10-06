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

//! WDB-456: Copy `i32` **local** into untyped `let` must not `.clone()`.
//!
//! Product `physics/physics_body.rs` check_collision:
//!   `let feet_y = min_y.clone()` / `let head_y = max_y.clone()`
//! WJ uses bare `min_y` / `max_y`.
//! Distinct from WDB-446 (i32 local into **typed** let), WDB-437 (i32 **formal** into let),
//! and WDB-455 (i32 local into **field** assign).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn ground_probe(pos_y: f32, size_y: f32) -> i32 {
    let min_y = (pos_y - size_y / 2.0) as i32
    let max_y = (pos_y + size_y / 2.0) as i32
    let feet_y = min_y
    let head_y = max_y
    feet_y + head_y
}
"#;

#[test]
fn wdb456_module_file_copy_i32_local_into_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-456 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-456 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("min_y.clone()") || rs.contains("max_y.clone()");
    assert!(
        !bad,
        "WDB-456 RED: Copy i32 local cloned into untyped let:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb456_search_roots() -> Vec<PathBuf> {
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
fn wdb456_tip_out_game_core_physics_body_i32_local_into_let_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb456_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/physics/physics_body.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/physics/physics_body.rs"));
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
                && (t.contains("feet_y = min_y.clone()") || t.contains("head_y = max_y.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-456: physics_body product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-456 RED: tip/product Copy i32 local into untyped let clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
