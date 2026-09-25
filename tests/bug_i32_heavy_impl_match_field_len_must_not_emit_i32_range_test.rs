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

//! P3.444: impl + `let node = match get_node` + `0..node.params.len()` must stay usize.
//!
//! Product `CsgScene::emit_node_instructions` after `let node = match self.get_node(node_id)`:
//! emitted `0_i32..node.params.len()` (E0308 expected i32, found usize) and
//! `self.emit_node_instructions(..., buf.clone())` into a demoted `&mut Vec<f32>`.
//!
//! Free-fn isolate (`for_zero_to_len_must_not_emit_i32_range`) does not cover match-bound
//! field access. A same-crate `len() -> i32` homonym must not poison `Vec::len`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct CsgNode {
    pub id: i32,
    pub params: Vec<f32>,
    pub left_child: i32,
    pub right_child: i32,
}

pub struct CsgScene {
    pub nodes: Vec<CsgNode>,
    pub root_id: i32,
}

pub struct NameLen {
    pub name: string,
}

impl NameLen {
    pub fn len(self) -> i32 {
        0
    }
}

impl CsgScene {
    pub fn get_node(self, id: i32) -> Option<CsgNode> {
        let mut i = 0
        while i < self.nodes.len() {
            if self.nodes[i].id == id {
                return Some(self.nodes[i])
            }
            i = i + 1
        }
        None
    }

    fn emit_node_instructions(self, node_id: i32, buf: Vec<f32>) {
        if node_id < 0 {
            return
        }
        let node_opt = self.get_node(node_id)
        let node = match node_opt {
            Some(n) => n,
            None => return,
        }
        let mut i = 0
        for _ in 0..node.params.len() {
            buf.push(node.params[i])
            i = i + 1
        }
        if node.left_child >= 0 {
            self.emit_node_instructions(node.left_child, buf)
        }
        if node.right_child >= 0 {
            self.emit_node_instructions(node.right_child, buf)
        }
    }

    pub fn to_instruction_buffer(self) -> Vec<f32> {
        let mut buf = Vec::new()
        self.emit_node_instructions(self.root_id, buf)
        buf
    }
}
"#;

fn bad_i32_len_range(rs: &str) -> bool {
    rs.contains("0_i32..") && rs.contains(".len()")
}

fn bad_mut_vec_clone(rs: &str) -> bool {
    let formal_mut = rs.contains("buf: &mut Vec") || rs.contains("buf: &mut ");
    let bad_call = rs.lines().any(|line| {
        line.contains("emit_node_instructions(") && line.contains("buf.clone()")
    });
    formal_mut && bad_call
}

#[test]
fn i32_heavy_impl_match_field_len_must_not_emit_i32_range() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.444 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.444 isolate lib.rs:\n{rs}");
    assert!(
        !bad_i32_len_range(rs),
        "P3.444: match-bound node.params.len() must not emit 0_i32..len():\n{rs}"
    );
    assert!(
        !bad_mut_vec_clone(rs),
        "P3.444: demoted &mut Vec must not receive buf.clone():\n{rs}"
    );
    test.cargo_check().expect("P3.444 cargo-check");
}

#[test]
fn p3444_tip_out_game_core_csg_must_not_emit_i32_len_range() {
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
        if bad_i32_len_range(&text) || bad_mut_vec_clone(&text) {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "P3.444: game-core/tip csg/scene missing");
    assert!(
        bad_paths.is_empty(),
        "P3.444 RED: i32 len range or buf.clone() into &mut Vec in:\n  {}",
        bad_paths.join("\n  ")
    );
}
