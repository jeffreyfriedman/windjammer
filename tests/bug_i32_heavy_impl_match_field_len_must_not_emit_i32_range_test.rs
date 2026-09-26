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

//! P3.444: product `CsgScene::emit_node_instructions` still emits
//!   `for i in 0_i32..node.params.len()` (E0308) and `buf.clone()` into
//!   demoted `&mut Vec<f32>`.
//!
//! P3.359 covers a free function `count_slots(node: Node) -> int`.
//! Product is an i32-heavy impl: `node_id: i32`, `primitive_type == 2`,
//! `get_node` → `Option`, then `for i in 0..node.params.len()`.
//! Engine also registers custom `len() -> i32` methods, which poisons
//! `consensus_return_is_usize("len")` when the match-bound receiver is
//! unresolved.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct PoisonLen {
    pub n: i32,
}

impl PoisonLen {
    pub fn len(self) -> i32 {
        self.n
    }
}

pub struct CsgNode {
    pub node_type: i32,
    pub primitive_type: i32,
    pub material_id: u8,
    pub params: Vec<f32>,
    pub left_child: i32,
    pub right_child: i32,
}

pub struct CsgScene {
    pub nodes: Vec<CsgNode>,
}

impl CsgScene {
    pub fn get_node(self, id: i32) -> Option<CsgNode> {
        if id < 0 {
            return None
        }
        if (id as usize) < self.nodes.len() {
            return Some(self.nodes[id as usize])
        }
        None
    }

    fn emit_instruction(self, buf: Vec<f32>, opcode: f32, material: u8, params: Vec<f32>) {
        buf.push(opcode)
        buf.push(material as f32)
        for i in 0..14 {
            if i < params.len() {
                buf.push(params[i])
            } else {
                buf.push(0.0)
            }
        }
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

        if node.node_type == 0 {
            let opcode = if node.primitive_type == 2 { 8.0 } else if node.primitive_type >= 3 { (node.primitive_type) as f32 } else { (node.primitive_type + 1) as f32 }
            let mut p = Vec::new()
            for i in 0..node.params.len() {
                p.push(node.params[i])
            }
            self.emit_instruction(buf, opcode, node.material_id, p)
        } else if node.node_type == 1 {
            self.emit_node_instructions(node.left_child, buf)
            self.emit_node_instructions(node.right_child, buf)
            self.emit_instruction(buf, 20.0, 0, Vec::new())
        }
    }
}
"#;

fn bad_i32_field_len_range(rs: &str) -> bool {
    rs.lines().any(|line| {
        let t = line.trim_start();
        !t.starts_with("//") && t.contains("0_i32..") && t.contains("params.len()")
    })
}

fn bad_owned_buf_clone_into_mut(rs: &str) -> bool {
    // Only the demoted `&mut` helper is illegal (`emit_instruction(buf.clone())`).
    // Recursive `emit_node_instructions(..., buf.clone())` is correct when that
    // sibling formal stayed owned `Vec<f32>`.
    let instr_mut = rs.contains("fn emit_instruction")
        && rs
            .lines()
            .any(|l| l.contains("fn emit_instruction") && l.contains("&mut Vec"));
    instr_mut
        && rs.lines().any(|line| {
            line.contains("emit_instruction(") && line.contains("buf.clone()")
        })
}

#[test]
fn i32_heavy_impl_match_field_len_must_not_emit_i32_range() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.444 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.444 MultiFile lib.rs:\n{rs}");
    assert!(
        !bad_i32_field_len_range(rs),
        "P3.444 RED: i32-heavy impl 0..node.params.len() must not emit 0_i32:\n{rs}"
    );
    assert!(
        !bad_owned_buf_clone_into_mut(rs),
        "P3.444 RED: demoted &mut Vec received owned buf.clone():\n{rs}"
    );
    test.cargo_check().expect("P3.444 cargo-check");
}

#[test]
fn p3444_tip_out_game_core_csg_must_not_emit_i32_len_range() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/csg/scene.rs");
    assert!(
        game.exists(),
        "P3.444: tip-out gen/csg/scene.rs missing at {}",
        game.display()
    );
    let text = std::fs::read_to_string(&game).expect("csg scene");
    assert!(
        !bad_i32_field_len_range(&text),
        "P3.444 RED: tip-out gen/csg/scene.rs still has 0_i32..params.len():\n{}",
        text.lines()
            .filter(|l| l.contains("0_i32") || l.contains("params.len()"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        !bad_owned_buf_clone_into_mut(&text),
        "P3.444 RED: tip-out gen/csg/scene.rs still passes buf.clone() into &mut Vec"
    );
}
