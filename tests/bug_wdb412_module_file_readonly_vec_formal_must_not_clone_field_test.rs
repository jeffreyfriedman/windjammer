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

//! WDB-412: read-only `for` over a `Vec` formal must borrow; call sites must not
//! `field.clone()` when WJ passes `pb.metas`.
//!
//! Product:
//!   `binding_metas_contain_name(metas: Vec<BindingMeta>, needle: &str)`
//!   `binding_metas_contain_name(pb.binding_metas.clone(), "screen_width")`
//! WJ is `contains_name(pb.binding_metas, "screen_width")` with `for m in metas`.
//! Distinct from WDB-410 (owned-self wither reconstruct clones) and WDB-407 (`Vec` `new`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Meta {
    pub name: string,
}

pub struct Pass {
    pub metas: Vec<Meta>,
}

fn contains_name(metas: Vec<Meta>, needle: string) -> bool {
    for m in metas {
        if m.name == needle {
            return true
        }
    }
    false
}

impl Pass {
    pub fn has_wh(self) -> bool {
        let mut pb = self
        if !contains_name(pb.metas, "w") {
            return false
        }
        if !contains_name(pb.metas, "h") {
            return false
        }
        true
    }
}
"#;

#[test]
fn wdb412_module_file_readonly_vec_formal_must_not_clone_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-412 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-412 MultiFile lib.rs:\n{rs}");
    let cloned_field = rs.contains("pb.metas.clone()")
        || rs.contains("contains_name(pb.metas.clone()");
    assert!(
        !cloned_field,
        "WDB-412 RED: read-only Vec formal forced field.clone() at call site:\n{rs}"
    );
    test.cargo_check().expect("WDB-412 cargo-check");
}

fn wdb412_search_roots() -> Vec<PathBuf> {
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
fn wdb412_tip_out_game_core_readonly_vec_formal_must_not_clone_field() {
    let mut paths = Vec::new();
    for dir in wdb412_search_roots() {
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
            !t.starts_with("//") && t.contains("pb.binding_metas.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-412: shader_graph_builder product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-412 RED: tip/product read-only Vec formal cloned field in:\n  {}",
        bad_paths.join("\n  ")
    );
}
