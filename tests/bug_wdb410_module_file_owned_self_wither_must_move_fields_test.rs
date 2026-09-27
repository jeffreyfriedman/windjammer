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

//! WDB-410: owned-self wither must move fields, not clone them.
//!
//! Product `PassBuilder::push_binding(self, …) -> PassBuilder`:
//!   `let mut new_bindings = self.bindings.clone();`
//!   `PassBuilder { graph: self.graph.clone(), bindings: new_bindings,
//!     binding_metas: self.binding_metas.clone(), dependencies: self.dependencies.clone(), … }`
//! WJ is `let mut new_bindings = self.bindings` then `PassBuilder { graph: self.graph, … }`.
//! Distinct from WDB-378 (indexed `src[i].clone().field` per field) and WDB-358 (`self.clone().method()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Graph {
    pub n: i32,
}

pub struct Builder {
    pub graph: Graph,
    pub items: Vec<i32>,
    pub label: string,
}

impl Builder {
    pub fn push(self, x: i32) -> Builder {
        let mut items = self.items
        items.push(x)
        Builder {
            graph: self.graph,
            items: items,
            label: self.label,
        }
    }
}
"#;

#[test]
fn wdb410_module_file_owned_self_wither_must_move_fields() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-410 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-410 MultiFile lib.rs:\n{rs}");
    let cloned_fields = rs.contains("self.items.clone()")
        || rs.contains("self.graph.clone()")
        || rs.contains("self.label.clone()");
    assert!(
        !cloned_fields,
        "WDB-410 RED: owned-self wither cloned fields instead of moving:\n{rs}"
    );
    test.cargo_check().expect("WDB-410 cargo-check");
}

fn wdb410_search_roots() -> Vec<PathBuf> {
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
fn wdb410_tip_out_game_core_owned_self_wither_must_move_fields() {
    let mut paths = Vec::new();
    for dir in wdb410_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/shader_graph_builder.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/rendering/shader_graph_builder.rs"));
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
                && (t.contains("self.graph.clone()")
                    || t.contains("self.bindings.clone()")
                    || t.contains("self.binding_metas.clone()")
                    || t.contains("self.dependencies.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-410: PassBuilder product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-410 RED: tip/product owned-self wither cloned fields in:\n  {}",
        bad_paths.join("\n  ")
    );
}
