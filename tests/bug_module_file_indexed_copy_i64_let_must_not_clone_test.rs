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

//! WDB-468: indexed Copy `i64` into a local `let` must not `.clone()`.
//!
//! Product `editor/csg.rs` `split_polygon_by_plane`:
//!   `let ti = types[i].clone(); let tj = types[j].clone();`
//! WJ uses bare `types[i]` / `types[j]` (untyped `2`/`1`/`0` pushed as i64).
//! Distinct from WDB-464 (indexed `u8` into push), WDB-466 (tuple field),
//! WDB-456 (plain i32 local reassign, not an index).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn accumulate(types: Vec<i64>) -> i64 {
    let mut acc: i64 = 0
    let mut i: int = 0
    while i < types.len() {
        let j = i + 1
        let ti = types[i]
        if j < types.len() {
            let tj = types[j]
            acc = acc | ti | tj
        } else {
            acc = acc | ti
        }
        i = i + 1
    }
    acc
}
"#;

#[test]
fn wdb468_module_file_indexed_copy_i64_let_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-468 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-468 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("types[i].clone()")
        || rs.contains("types[j].clone()")
        || rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-468 RED: indexed Copy i64 cloned into let:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb468_search_roots() -> Vec<PathBuf> {
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
fn wdb468_tip_out_game_core_csg_indexed_i64_let_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb468_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/csg.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/editor/csg.rs"));
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
                && (t.contains("types[i].clone()") || t.contains("types[j].clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-468: csg product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-468 RED: tip/product indexed Copy i64 cloned into let:\n  {}",
        bad_paths.join("\n  ")
    );
}
