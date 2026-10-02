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

//! WDB-430: Copy unit-enum **field** of a local/loop binding must not `.clone()`
//! into an owned formal.
//!
//! Product `rendering/shader_graph_builder.rs` / `shader_graph_compiler.rs`:
//!   `is_storage_write(binding.binding_type.clone())`
//! WJ is `is_storage_write(binding.binding_type)`.
//! Distinct from WDB-375 (nested index clone **chain**
//! `].clone().bindings[j].clone().binding_type.clone()`), WDB-402 (`HostType::F32.clone()`
//! path), and WDB-392 (`Direction::PosX.clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum BindingType {
    Read,
    Write,
}

pub struct Binding {
    pub binding_type: BindingType,
}

pub fn is_write(t: BindingType) -> bool {
    match t {
        BindingType::Write => true,
        BindingType::Read => false,
    }
}

pub fn any_write(bindings: Vec<Binding>) -> bool {
    for binding in bindings {
        if is_write(binding.binding_type) {
            return true
        }
    }
    false
}
"#;

#[test]
fn wdb430_module_file_copy_enum_field_into_owned_formal_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-430 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-430 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("binding_type.clone()");
    assert!(
        !bad,
        "WDB-430 RED: Copy enum field cloned into owned formal:\n{rs}"
    );
    test.cargo_check().expect("WDB-430 cargo-check");
}

fn wdb430_search_roots() -> Vec<PathBuf> {
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
fn wdb430_tip_out_game_core_binding_type_field_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb430_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/shader_graph_builder.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/shader_graph_compiler.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/rendering/shader_graph_builder.rs"),
        );
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/rendering/shader_graph_compiler.rs"),
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
            !t.starts_with("//") && t.contains("binding.binding_type.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-430: shader_graph product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-430 RED: tip/product cloned Copy BindingType field:\n  {}",
        bad_paths.join("\n  ")
    );
}
