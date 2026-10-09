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

//! WDB-476: an `i32` compared with `< 0` must not widen the zero to `i64`.
//!
//! Product `procedural/simplex_noise.rs` `new`:
//!   `let val = (... ) as i32` then `if val < (0_i64 as i64)`.
//! WJ is `if val < 0` after `as i32`.
//! Distinct from WDB-475 (an untyped corner offset inferred as `i64`) and
//! from P3.770 (a tuple array mixing `0_i64` beside `i32`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn abs_perm(seed: i64, i: i32) -> i32 {
    let val = ((seed * (i as i64 + 1)) % 256) as i32
    if val < 0 { -val } else { val }
}
"#;

#[test]
fn wdb476_module_file_i32_compared_to_zero_must_not_widen() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-476 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-476 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("0_i64");
    assert!(
        !bad,
        "WDB-476 RED: i32 compared to a widened zero:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb476_search_roots() -> Vec<PathBuf> {
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
fn wdb476_tip_out_simplex_i32_lt_zero_must_not_widen() {
    let mut paths = Vec::new();
    for dir in wdb476_search_roots() {
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
            !t.starts_with("//") && t.contains("val < (0_i64")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-476: simplex_noise product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-476 RED: tip/product i32 compared to a widened zero:\n  {}",
        bad_paths.join("\n  ")
    );
}
