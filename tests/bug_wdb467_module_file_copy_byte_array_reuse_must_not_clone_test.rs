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

//! WDB-467: Copy `[u8; 4]` from `to_le_bytes` reused in `from_le_bytes` must not `.clone()`.
//!
//! Product `rendering/hybrid_renderer.rs` `pack_visibility_params`:
//!   `from_le_bytes(zero_bits.clone())` then `from_le_bytes(zero_bits)`
//! WJ uses bare `zero_bits` both times.
//! Distinct from WDB-464 (indexed `u8` element into `Vec::push`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn pack_zeros() -> Vec<f32> {
    let mut data: Vec<f32> = Vec::new()
    let zero_bits = 0u32.to_le_bytes()
    data.push(f32::from_le_bytes(zero_bits))
    data.push(f32::from_le_bytes(zero_bits))
    data
}
"#;

#[test]
fn wdb467_module_file_copy_byte_array_reuse_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-467 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-467 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("zero_bits.clone()");
    assert!(
        !bad,
        "WDB-467 RED: Copy byte array cloned into from_le_bytes:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb467_search_roots() -> Vec<PathBuf> {
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
fn wdb467_tip_out_game_core_hybrid_renderer_byte_array_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb467_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/hybrid_renderer.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/rendering/hybrid_renderer.rs"),
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
            !t.starts_with("//") && t.contains("from_le_bytes(") && t.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-467: hybrid_renderer product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-467 RED: tip/product Copy byte array cloned into from_le_bytes:\n  {}",
        bad_paths.join("\n  ")
    );
}
