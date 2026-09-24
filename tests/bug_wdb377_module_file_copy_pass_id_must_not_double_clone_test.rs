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

//! WDB-377: indexed Copy PassId must not emit `].clone().pass_id.clone()`.
//!
//! Product tip game-core `rendering/shader_graph_compiler.rs`:
//!   `sorted[i].clone().pass_id.clone()` / `sorted[j].clone().shader_file.clone()`
//! Prefer `sorted[i].pass_id` (Copy enum). Twin of WDB-374/370.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum PassId {
    A,
    B,
}

pub struct Pass {
    pub pass_id: PassId,
}

pub fn collect_ids(passes: Vec<Pass>) -> Vec<PassId> {
    let mut out = Vec::new()
    let mut i = 0
    while i < passes.len() {
        out.push(passes[i].pass_id)
        i = i + 1
    }
    out
}
"#;

#[test]
fn wdb377_module_file_copy_pass_id_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-377 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-377 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().pass_id") || rs.contains(".pass_id.clone()");
    assert!(
        !bad,
        "WDB-377 RED: indexed Copy PassId double-cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-377 cargo-check");
}

#[test]
fn wdb377_tip_out_game_core_shader_must_not_double_clone_pass_id() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/shader_graph_compiler.rs"),
        tip.join("shader_graph_compiler.rs"),
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
                && (line.contains("].clone().pass_id.clone()")
                    || line.contains("].clone().pass_id"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-377: shader_graph_compiler product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-377 RED: tip/product indexed pass_id double-clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
