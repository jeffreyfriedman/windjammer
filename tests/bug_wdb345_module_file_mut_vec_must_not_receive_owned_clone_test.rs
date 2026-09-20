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

//! WDB-345: `&mut Vec` formal must not receive owned `buf.clone()`.
//!
//! Product tip game-core `csg/scene.rs`:
//!   `emit_node_instructions(..., buf.clone())` / `emit_instruction(buf.clone(), …)`
//!   with `buf: &mut Vec<f32>` → E0308 (expected `&mut Vec`, found `Vec`).
//! Twin of WDB-335/338 polarity (owned into borrowed/mut). Prefer `&mut buf`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn emit_instruction(buf: Vec<f32>, opcode: f32) {
    buf.push(opcode)
}

pub fn emit_tree(buf: Vec<f32>) {
    emit_instruction(buf, 1.0)
    emit_instruction(buf, 2.0)
}
"#;

#[test]
fn wdb345_module_file_mut_vec_must_not_receive_owned_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-345 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-345 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("buf.clone()")
        || rs.contains("emit_instruction(buf.clone()");
    assert!(
        !bad,
        "WDB-345 RED: mut Vec received owned buf.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-345 cargo-check");
}

#[test]
fn wdb345_tip_out_game_core_csg_must_not_pass_owned_buf_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("scene.rs"),
        tip.join("csg/scene.rs"),
        game.join("gen/csg/scene.rs"),
        game.join("csg/scene.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("csg scene");
        let formal_mut = text.contains("buf: &mut Vec") || text.contains("buf: &mut ");
        let bad_call = text.lines().any(|line| {
            (line.contains("emit_node_instructions(") || line.contains("emit_instruction("))
                && line.contains("buf.clone()")
        });
        if formal_mut && bad_call {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-345: game-core/tip csg/scene missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-345 RED: tip/product passes buf.clone() into &mut Vec in:\n  {}",
        bad_paths.join("\n  ")
    );
}
