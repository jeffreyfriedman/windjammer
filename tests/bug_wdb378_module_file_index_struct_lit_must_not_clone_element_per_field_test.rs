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

//! WDB-378: reconstructing a struct from indexed fields must not `src[i].clone().field` per field.
//!
//! Product tip game-core `rendering/shader_graph_builder.rs`:
//!   `PassDefinition { pass_id: pass_defs[current].clone().pass_id, shader: pass_defs[current].clone().shader,
//!     bindings: pass_defs[current].clone().bindings.clone(), … dispatch_x: pass_defs[current].clone().dispatch_x, … }`
//! Prefer one `let p = pass_defs[current].clone()` or field access without per-field element clone.
//! Twin of WDB-370/375.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Pass {
    pub id: i32,
    pub bindings: Vec<i32>,
    pub dispatch_x: u32,
}

pub fn copy_pass(passes: Vec<Pass>, i: usize) -> Pass {
    Pass {
        id: passes[i].id,
        bindings: passes[i].bindings,
        dispatch_x: passes[i].dispatch_x,
    }
}
"#;

#[test]
fn wdb378_module_file_index_struct_lit_must_not_clone_element_per_field() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-378 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-378 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().id")
        || rs.contains("].clone().bindings")
        || rs.contains("].clone().dispatch_x");
    assert!(
        !bad,
        "WDB-378 RED: per-field indexed element clone in struct lit:\n{rs}"
    );
    test.cargo_check().expect("WDB-378 cargo-check");
}

#[test]
fn wdb378_tip_out_game_core_builder_must_not_clone_element_per_field() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/shader_graph_builder.rs"),
        tip.join("shader_graph_builder.rs"),
        game.join("gen/rendering/shader_graph_builder.rs"),
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
                && line.contains("].clone().pass_id")
                && line.contains("].clone().bindings")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-378: shader_graph_builder product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-378 RED: tip/product per-field index clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
