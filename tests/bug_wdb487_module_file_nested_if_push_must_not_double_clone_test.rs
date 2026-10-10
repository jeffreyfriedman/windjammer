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

//! DB-487: a struct pushed on a nested `if` arm, while a sibling arm reads
//! its fields, must not `.clone().clone()`. One clone on the push-only arm
//! is enough.
//!
//! Product `editor/csg.rs`:
//!   `back.push(poly.clone().clone())`
//!   while the spanning else reads `poly.vertices`
//! WJ is `back.push(poly)` on the `polygon_type == 2` arm.
//! Distinct from DB-478 (every exclusive arm only moves the `for` binding,
//! so no clone is required) and DB-474 (one last-use local, no sibling read).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Poly {
    pub kind: int,
    pub verts: Vec<int>,
}

pub fn split(poly: Poly, kind: int) -> int {
    let mut front = Vec::new()
    let mut back = Vec::new()
    if kind == 1 {
        front.push(poly)
    } else {
        if kind == 2 {
            back.push(poly)
        } else {
            let n = poly.verts.len()
            front.push(poly)
            let _kept = n
        }
    }
    front.len() + back.len()
}
"#;

#[test]
fn wdb487_module_file_nested_if_push_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("DB-487 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("DB-487 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "DB-487 RED: nested-if push double-cloned the struct:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb487_search_roots() -> Vec<PathBuf> {
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
fn wdb487_tip_out_csg_poly_push_must_not_double_clone() {
    let mut paths = Vec::new();
    for dir in wdb487_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/editor/csg.rs"));
        paths.push(dir.join("game-host/game-host-core/gen/editor/csg.rs"));
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
            !t.starts_with("//") && t.contains("poly.clone().clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "DB-487: csg product files missing");
    assert!(
        bad_paths.is_empty(),
        "DB-487 RED: tip/product nested-if poly double-cloned into push:\n  {}",
        bad_paths.join("\n  ")
    );
}
