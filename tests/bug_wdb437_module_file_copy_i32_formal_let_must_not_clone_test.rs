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

//! WDB-437: Copy `i32` formal into local `let` must not `.clone()`.
//!
//! Product `scene/station_geometry.rs`:
//!   `let mut r = radius.clone();`
//! WJ is `let mut r = radius`.
//! Distinct from WDB-427 (Copy **field** into let), WDB-428 (local in println),
//! and WDB-434 (const into **call** formals).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn shrink_radius(radius: i32, dy: i32, height: i32) -> i32 {
    let mut r = radius
    if dy < 2 || dy > height - 3 {
        r = radius - 1
        if r < 1 {
            r = 1
        }
    }
    r
}
"#;

#[test]
fn wdb437_module_file_copy_i32_formal_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-437 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-437 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("radius.clone()");
    assert!(
        !bad,
        "WDB-437 RED: Copy i32 formal cloned into let:\n{rs}"
    );
    test.cargo_check().expect("WDB-437 cargo-check");
}

fn wdb437_search_roots() -> Vec<PathBuf> {
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
fn wdb437_tip_out_game_core_station_geometry_radius_let_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb437_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/scene/station_geometry.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/scene/station_geometry.rs"),
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
            !t.starts_with("//") && t.contains("radius.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-437: station_geometry product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-437 RED: tip/product Copy i32 formal let clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
