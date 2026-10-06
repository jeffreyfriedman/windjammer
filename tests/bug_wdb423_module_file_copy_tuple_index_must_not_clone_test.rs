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

//! WDB-423: Copy tuple array index destructure must not `.clone()`.
//!
//! Product `fps_camera.rs` / `tps_camera.rs`:
//!   `let (ox, oz) = offsets[(i as usize)].clone();`
//! WJ is `let (ox, oz) = offsets[i]`.
//! First array element may be `(-1, 0)` — Neg must not break Copy inference.
//! Distinct from WDB-363 (indexed tuple **field**), WDB-422 (f32 arith),
//! and WDB-393 (local i32).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn pick(i: i32) -> f32 {
    let offsets = [
        (1.0, 0.0, 0.0),
        (-1.0, 0.0, 0.0),
    ]
    let (ox, oy, oz) = offsets[i]
    ox + oy + oz
}

/// Product shape (tps_camera): i32 neighbor offsets with leading Neg literal.
pub fn collides(cx: i32, cz: i32) -> bool {
    let offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    let mut i = 0
    while i < 4 {
        let (ox, oz) = offsets[i]
        let check_x = cx + ox
        let check_z = cz + oz
        if check_x == 0 && check_z == 0 {
            return true
        }
        i = i + 1
    }
    false
}
"#;

#[test]
fn wdb423_module_file_copy_tuple_index_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-423 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-423 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "WDB-423 RED: Copy tuple array index cloned on destructure:\n{rs}"
    );
    test.cargo_check().expect("WDB-423 cargo-check");
}

fn wdb423_search_roots() -> Vec<PathBuf> {
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
fn wdb423_tip_out_game_core_offsets_index_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb423_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/fps_camera.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/camera/fps_camera.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/camera/tps_camera.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/camera/fps_camera.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/camera/tps_camera.rs"));
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
            !t.starts_with("//") && t.contains("offsets[") && t.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-423: camera offsets product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-423 RED: tip/product cloned Copy offsets tuple in:\n  {}",
        bad_paths.join("\n  ")
    );
}
