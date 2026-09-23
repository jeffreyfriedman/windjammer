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

//! WDB-375: nested index Copy field must not emit
//! `].clone().bindings[j].clone().binding_type.clone()`.
//!
//! Product tip game-core `rendering/shader_graph_compiler.rs`:
//!   `is_storage_read(sorted[idx].clone().bindings[bi].clone().binding_type.clone())`
//! WJ source is `sorted[idx].bindings[bi].binding_type` (Copy enum). Twin of WDB-370/374.

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

pub struct Pass {
    pub bindings: Vec<Binding>,
    pub name: string,
}

pub fn is_read(t: BindingType) -> bool {
    match t {
        BindingType::Read => true,
        BindingType::Write => false,
    }
}

pub struct Graph {
    pub passes: Vec<Pass>,
}

impl Graph {
    pub fn first_is_read(self, idx: usize, bi: usize) -> bool {
        is_read(self.passes[idx].bindings[bi].binding_type)
    }
}
"#;

#[test]
fn wdb375_module_file_nested_index_must_not_clone_chain() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-375 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-375 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().bindings")
        || rs.contains("].clone().binding_type")
        || rs.contains(".binding_type.clone()");
    assert!(
        !bad,
        "WDB-375 RED: nested index Copy field clone chain:\n{rs}"
    );
    test.cargo_check().expect("WDB-375 cargo-check");
}

#[test]
fn wdb375_tip_out_game_core_shader_graph_must_not_clone_binding_chain() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/shader_graph_compiler.rs"),
        game.join("gen/rendering/shader_graph_compiler.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//")
                && (line.contains("].clone().bindings")
                    || line.contains(".binding_type.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-375: shader_graph_compiler product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-375 RED: tip/product nested binding clone chain in:\n  {}",
        bad_paths.join("\n  ")
    );
}
