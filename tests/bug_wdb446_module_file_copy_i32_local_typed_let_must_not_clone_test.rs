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

//! WDB-446: Copy `i32` **local** into typed `let` must not `.clone()`.
//!
//! Product `voxel/meshing.rs`:
//!   `let mut slice_count: i32 = gd.clone()`
//!   `let mut x: i32 = u.clone()`
//! WJ uses bare `gd` / `u`.
//! Distinct from WDB-437 (i32 **formal** into let), WDB-442 (u32 **local** into let),
//! and WDB-445 (f32 **formal** into let).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn greedy_axis(gw: i32, gh: i32, gd: i32, axis: i32) -> i32 {
    let mut slice_count: i32 = gd
    if axis == 0 {
        slice_count = gw
    }
    if axis == 1 {
        slice_count = gh
    }
    let mut u: i32 = 0
    let mut total: i32 = 0
    while u < slice_count {
        let mut x: i32 = u
        let mut y: i32 = u
        total = total + x + y
        u = u + 1
    }
    total
}
"#;

#[test]
fn wdb446_module_file_copy_i32_local_typed_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-446 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-446 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("gd.clone()")
        || rs.contains("gw.clone()")
        || rs.contains("gh.clone()")
        || rs.contains("u.clone()");
    assert!(
        !bad,
        "WDB-446 RED: Copy i32 local cloned into typed let:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb446_search_roots() -> Vec<PathBuf> {
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
fn wdb446_tip_out_game_core_meshing_i32_local_typed_let_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb446_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/voxel/meshing.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/voxel/meshing.rs"));
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
                && (t.contains("gd.clone()")
                    || t.contains("let mut x: i32 = u.clone()")
                    || t.contains("let mut y: i32 = v.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-446: meshing product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-446 RED: tip/product Copy i32 local into typed let clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
