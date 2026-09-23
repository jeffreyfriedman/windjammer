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

//! WDB-376: indexed Copy u32 field must not emit `].clone().buffer_id` / `.buffer_id.clone()`.
//!
//! Product tip game-core `rendering/shader_graph_compiler.rs`:
//!   `lifetimes[li].clone().buffer_id`
//! WJ source is `lifetimes[li].buffer_id` (Copy u32). Twin of WDB-370 / WDB-343.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Lifetime {
    pub buffer_id: u32,
    pub name: string,
}

pub fn takes_id(id: u32) -> u32 {
    id
}

pub struct Analysis {
    pub lifetimes: Vec<Lifetime>,
}

impl Analysis {
    pub fn first_id(self, i: usize) -> u32 {
        takes_id(self.lifetimes[i].buffer_id)
    }
}
"#;

#[test]
fn wdb376_module_file_copy_u32_index_field_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-376 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-376 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().buffer_id") || rs.contains(".buffer_id.clone()");
    assert!(
        !bad,
        "WDB-376 RED: indexed Copy u32 field cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-376 cargo-check");
}

#[test]
fn wdb376_tip_out_game_core_shader_graph_must_not_clone_buffer_id() {
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
                && (line.contains("].clone().buffer_id") || line.contains(".buffer_id.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-376: shader_graph_compiler product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-376 RED: tip/product indexed buffer_id clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
