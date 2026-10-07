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

//! WDB-459: Copy **struct** local into `Vec::push` must not `.clone()`.
//!
//! Product `editor/uv_unwrap_algorithm.rs`:
//!   `let a = tri_norm.a; all_corners.push(a.clone()); uvs.push(a.clone())`
//! WJ uses bare `a` / `b` / `c`.
//! Distinct from WDB-457 (Copy **newtype** into push), WDB-458 (newtype into
//! owned formal), WDB-440 (f32 local into **struct lit**), WDB-438 (i32 formal
//! into tuple lit for push).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct UvCoord {
    pub u: f32,
    pub v: f32,
}

pub struct TriNorm {
    pub a: UvCoord,
    pub b: UvCoord,
    pub c: UvCoord,
}

pub fn collect_corners(tri_norm: TriNorm) -> Vec<UvCoord> {
    let a = tri_norm.a
    let b = tri_norm.b
    let c = tri_norm.c
    let mut all_corners: Vec<UvCoord> = Vec::new()
    all_corners.push(a)
    all_corners.push(b)
    all_corners.push(c)
    all_corners
}
"#;

#[test]
fn wdb459_module_file_copy_struct_local_vec_push_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-459 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-459 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("a.clone()") || rs.contains("b.clone()") || rs.contains("c.clone()");
    assert!(
        !bad,
        "WDB-459 RED: Copy struct local cloned into Vec::push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb459_search_roots() -> Vec<PathBuf> {
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
fn wdb459_tip_out_game_core_uv_unwrap_copy_struct_vec_push_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb459_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/uv_unwrap_algorithm.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/uv_island_packing.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/editor/uv_unwrap_algorithm.rs"),
        );
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/editor/uv_island_packing.rs"));
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
                && (t.contains("push(a.clone())")
                    || t.contains("push(b.clone())")
                    || t.contains("push(c.clone())")
                    || t.contains("push(fa.clone())")
                    || t.contains("push(fb.clone())")
                    || t.contains("push(fc.clone())"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-459: uv unwrap/packing product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-459 RED: tip/product Copy struct local into Vec::push clone:\n  {}",
        bad_paths.join("\n  ")
    );
}
