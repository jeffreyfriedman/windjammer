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

//! WDB-481: a `string` local pushed into a `Vec` and then used again must not
//! `.clone().clone()`. One clone into `push` is enough.
//!
//! Product `rendering/material_node_graph.rs`:
//!   `node_vars.push(var_name.clone().clone())`
//!   then `format!(..., var_name, expr)`
//! WJ is `node_vars.push(var_name)` and then `format!("    let {} = {};\n", var_name, expr)`.
//! Distinct from WDB-479 (a reused `string` map key), WDB-480 (a loop-reused
//! struct argument), and WDB-474 (a last-use struct into `push`, where no
//! clone is required).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn compile(ids: Vec<int>) -> string {
    let mut node_vars = Vec::new()
    let mut code = ""
    for id in ids {
        let var_name = format!("node_{}", id)
        node_vars.push(var_name)
        code = code + format!("    let {} = 1;\n", var_name)
    }
    let _kept = node_vars
    code
}
"#;

#[test]
fn wdb481_module_file_reused_string_push_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-481 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-481 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "WDB-481 RED: reused string double-cloned into push:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb481_search_roots() -> Vec<PathBuf> {
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
fn wdb481_tip_out_material_var_name_push_must_not_double_clone() {
    let mut paths = Vec::new();
    for dir in wdb481_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/material_node_graph.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/rendering/material_node_graph.rs"),
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
            !t.starts_with("//") && t.contains("var_name.clone().clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-481: material node graph product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-481 RED: tip/product reused string double-cloned into push:\n  {}",
        bad_paths.join("\n  ")
    );
}
