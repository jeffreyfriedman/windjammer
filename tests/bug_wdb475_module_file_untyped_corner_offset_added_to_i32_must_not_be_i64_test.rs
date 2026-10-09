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

//! WDB-475: an untyped `0`/`1` corner offset later added to an `i32` must not
//! become `i64`.
//!
//! Product `procedural/simplex_noise.rs` `noise3d`:
//!   `let mut i1 = 0_i64` then `ii as i64 + i1` where `ii` is `i32`.
//! WJ is `let mut i1 = 0` and `ii + i1`, and `i1` is also cast with `as f32`.
//! Distinct from P3.755 (`0` plus `Vec::len()`) and from the neighbor-offset
//! tuple literal that mixes `0_i64` inside `(i32, i32)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn corner(ii: i32, x0: f32, y0: f32) -> f32 {
    let mut i1 = 0
    let mut j1 = 0
    if x0 >= y0 {
        i1 = 1
        j1 = 0
    } else {
        i1 = 0
        j1 = 1
    }
    let shifted = x0 - i1 as f32
    let idx = ii + i1 + j1
    shifted + idx as f32
}
"#;

#[test]
fn wdb475_module_file_untyped_corner_offset_added_to_i32_must_not_be_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-475 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-475 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("0_i64") || rs.contains("as i64");
    assert!(
        !bad,
        "WDB-475 RED: untyped corner offset widened to i64:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb475_search_roots() -> Vec<PathBuf> {
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
fn wdb475_tip_out_simplex_corner_offset_must_not_be_i64() {
    let mut paths = Vec::new();
    for dir in wdb475_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/procedural/simplex_noise.rs"));
        paths.push(dir.join(
            "windjammer-game/windjammer-game-core/gen/procedural/simplex_noise.rs",
        ));
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
            !t.starts_with("//") && (t.contains("let mut i1 = 0_i64") || t.contains("as i64 + i1"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-475: simplex_noise product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-475 RED: tip/product corner offset widened to i64:\n  {}",
        bad_paths.join("\n  ")
    );
}
