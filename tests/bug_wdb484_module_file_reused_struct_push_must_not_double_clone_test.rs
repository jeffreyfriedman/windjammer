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

//! WDB-484: a struct parameter pushed into a `Vec` and then passed to a later
//! call must not `.clone().clone()` on the push. One clone into `push` is enough.
//!
//! Product `vgs/lod_generator.rs` `generate_lods`:
//!   `base.push(cluster.clone().clone())`
//!   then `simplify_cluster(cluster.clone(), ratio)`
//! WJ is `base.push(cluster)` and later `simplify_cluster(cluster, ratio)`.
//! Distinct from WDB-481 (a fresh string local pushed and formatted) and
//! WDB-480 (the loop parameter double-cloned into a call).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Cluster {
    pub id: int,
    pub name: string,
}

pub fn simplify(cluster: Cluster, ratio: f32) -> Cluster {
    Cluster { id: cluster.id, name: cluster.name }
}

pub fn generate(cluster: Cluster) -> Vec<Cluster> {
    let mut base = Vec::new()
    base.push(cluster)
    let simplified = simplify(cluster, 0.5)
    let mut out = Vec::new()
    out.push(simplified)
    let _kept = base
    out
}
"#;

#[test]
fn wdb484_module_file_reused_struct_push_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-484 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-484 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "WDB-484 RED: reused struct double-cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb484_search_roots() -> Vec<PathBuf> {
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
fn wdb484_tip_out_lod_cluster_push_must_not_double_clone() {
    let mut paths = Vec::new();
    for dir in wdb484_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/vgs/lod_generator.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/vgs/lod_generator.rs"));
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
            !t.starts_with("//") && t.contains("cluster.clone().clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-484: lod generator product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-484 RED: tip/product reused cluster double-cloned into push:\n  {}",
        bad_paths.join("\n  ")
    );
}
