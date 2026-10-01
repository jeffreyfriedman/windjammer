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

//! WDB-428: Copy local `i32` in format/println args must not `.clone()`.
//!
//! Product `voxel/svo64_convert.rs` / `svo_convert.rs`:
//!   `println!(..., max_size.clone())`
//! WJ is `println(..., max_size)`.
//! Distinct from WDB-393 (local **assign** `x.clone()`), WDB-427 (field let),
//! and WDB-426 (const `u32`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn log_size(w: i32, h: i32, d: i32) {
    let max_size = w
    println("grid {}x{}x{} size={}", w, h, d, max_size)
}
"#;

#[test]
fn wdb428_module_file_copy_local_in_println_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-428 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-428 MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "WDB-428 RED: Copy local cloned in println/format:\n{rs}"
    );
    test.cargo_check().expect("WDB-428 cargo-check");
}

fn wdb428_search_roots() -> Vec<PathBuf> {
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
fn wdb428_tip_out_game_core_max_size_println_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb428_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/voxel/svo64_convert.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/voxel/svo_convert.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/svo_convert.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/voxel/svo64_convert.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/voxel/svo_convert.rs"));
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
            !t.starts_with("//") && t.contains("max_size.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-428: svo convert product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-428 RED: tip/product cloned Copy max_size in println:\n  {}",
        bad_paths.join("\n  ")
    );
}
