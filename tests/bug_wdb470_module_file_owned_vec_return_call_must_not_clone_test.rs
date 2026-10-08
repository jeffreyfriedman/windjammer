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

//! WDB-470: an owned `Vec` return passed into an owned formal must not `.clone()`.
//!
//! Product `editor/csg.rs` `csg_union` / `csg_subtract` / `csg_intersect`:
//!   `build_bsp_from_polygons(clone_polygons(&a).clone())`
//! WJ is `build_bsp_from_polygons(clone_polygons(a))`.
//! `clone_polygons` already returns `Vec<CsgPolygon>`.
//! Distinct from Copy-element `.clone()` gates (WDB-464 through WDB-469).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Poly {
    pub verts: Vec<int>,
}

fn clone_polys(polys: Vec<Poly>) -> Vec<Poly> {
    let mut out = Vec::new()
    for i in 0..polys.len() {
        let p = polys[i]
        out.push(p)
    }
    out
}

fn build_tree(polygons: Vec<Poly>) -> int {
    polygons.len()
}

pub fn combine(a: Vec<Poly>, b: Vec<Poly>) -> int {
    let mut ta = build_tree(clone_polys(a))
    let mut tb = build_tree(clone_polys(b))
    ta + tb
}
"#;

#[test]
fn wdb470_module_file_owned_vec_return_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-470 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-470 MultiFile lib.rs:\n{rs}");
    let bad = rs.lines().any(|line| line.contains("clone_polys(") && line.contains(".clone()"));
    assert!(
        !bad,
        "WDB-470 RED: owned Vec return cloned at the call:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb470_search_roots() -> Vec<PathBuf> {
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
fn wdb470_tip_out_game_core_csg_owned_return_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb470_search_roots() {
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
            !t.starts_with("//") && t.contains("clone_polygons(") && t.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-470: csg product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-470 RED: tip/product owned Vec return cloned at the call:\n  {}",
        bad_paths.join("\n  ")
    );
}
