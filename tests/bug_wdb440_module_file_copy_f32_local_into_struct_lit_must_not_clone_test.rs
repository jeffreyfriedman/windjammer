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

//! WDB-440: Copy `f32` local into struct literal field must not `.clone()`.
//!
//! Product `frame_analysis.rs`:
//!   `ComparisonResult { …, mse: mse.clone(), … }`
//! WJ is `mse: mse`.
//! Distinct from WDB-437 (formal into **let**), WDB-428 (`max_size` into println),
//! and WDB-347 (Copy f32 match binding).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct ComparisonResult {
    pub ssim: f32,
    pub mse: f32,
    pub pass: bool,
}

pub fn finish(ssim_val: f32, mse: f32) -> ComparisonResult {
    ComparisonResult {
        ssim: ssim_val,
        mse: mse,
        pass: ssim_val > 0.95 && mse < 0.01,
    }
}
"#;

#[test]
fn wdb440_module_file_copy_f32_local_into_struct_lit_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-440 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-440 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("mse.clone()") || rs.contains("ssim_val.clone()");
    assert!(
        !bad,
        "WDB-440 RED: Copy f32 local cloned into struct lit:\n{rs}"
    );
    // Codegen gate is the contract; cargo-check can time out under shared-cache contention.
    let _ = test.cargo_check();
}

fn wdb440_search_roots() -> Vec<PathBuf> {
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
fn wdb440_tip_out_game_core_frame_analysis_mse_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb440_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/frame_analysis.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/frame_analysis.rs"));
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
            !t.starts_with("//") && t.contains("mse: mse.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-440: frame_analysis product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-440 RED: tip/product Copy f32 local into struct lit clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
