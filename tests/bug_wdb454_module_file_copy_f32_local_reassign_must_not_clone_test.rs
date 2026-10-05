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

//! WDB-454: Copy `f32` **local** into **local reassignment** must not `.clone()`.
//!
//! Product `frame_analysis.rs` histogram loop:
//!   `min_val = lum.clone()` / `max_val = lum.clone()`
//! WJ uses bare `lum`.
//! Distinct from WDB-452 (f32 **formal** local reassign), WDB-445 (f32 formal into **let**),
//! WDB-440 (f32 local into **struct literal**), and WDB-448 (f32 formal into **field**).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn track_local_extrema(samples: Vec<f32>) -> f32 {
    let mut min_val = 1000.0
    let mut max_val = -1000.0
    let mut i = 0
    while i < samples.len() {
        let lum = samples[i]
        if lum < min_val {
            min_val = lum
        }
        if lum > max_val {
            max_val = lum
        }
        i = i + 1
    }
    max_val - min_val
}
"#;

#[test]
fn wdb454_module_file_copy_f32_local_reassign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-454 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-454 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("lum.clone()");
    assert!(
        !bad,
        "WDB-454 RED: Copy f32 local cloned into local reassignment:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb454_search_roots() -> Vec<PathBuf> {
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
fn wdb454_tip_out_game_core_frame_analysis_f32_local_reassign_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb454_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/frame_analysis.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/object_pool/frame_analysis.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/frame_analysis.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/object_pool/frame_analysis.rs"),
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
            !t.starts_with("//") && t.contains("lum.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-454: frame_analysis product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-454 RED: tip/product Copy f32 local into local reassignment clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
