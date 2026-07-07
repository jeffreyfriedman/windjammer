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
))]

//! TDD: Multi-file copy type auto-borrow.
//!
//! When a Copy struct (e.g. NodeId) is defined in one file and used in another,
//! the codegen must know it's Copy and NOT add & at call sites.
//! This is the root cause of ~20+ E0308 errors in windjammer-game.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn cross_file_copy_struct_no_autoborrow() {
    let mut test = MultiFileTest::new();

    test.add_file(
        "node_id.wj",
        r#"
pub struct NodeId {
    pub value: int,
}
"#,
    );

    test.add_file(
        "tree.wj",
        r#"
use crate::node_id::NodeId

pub struct BehaviorTree {
    pub nodes: Vec<NodeId>,
}

impl BehaviorTree {
    pub fn find_node_index(self, id: NodeId) -> int {
        let mut idx = 0
        for node in self.nodes {
            if node.value == id.value {
                return idx
            }
            idx = idx + 1
        }
        -1
    }

    pub fn add_child(self, parent: NodeId, child: NodeId) {
        let idx = self.find_node_index(parent)
    }

    pub fn has_node(self, id: NodeId) -> bool {
        self.find_node_index(id) >= 0
    }
}
"#,
    );

    test.assert_compiles_without_error();
}

#[test]
fn cross_file_copy_vec3_no_autoborrow() {
    let mut test = MultiFileTest::new();

    test.add_file(
        "vec3.wj",
        r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
"#,
    );

    test.add_file(
        "perception.wj",
        r#"
use crate::vec3::Vec3

pub struct Perception {
    pub range: f32,
}

impl Perception {
    pub fn is_in_sight_cone(self, origin: Vec3, forward: Vec3, target: Vec3) -> bool {
        let dx = target.x - origin.x
        dx > 0.0
    }

    pub fn check(self, npc_pos: Vec3, npc_forward: Vec3, target_pos: Vec3) {
        let result = self.is_in_sight_cone(npc_pos, npc_forward, target_pos)
    }
}
"#,
    );

    test.assert_compiles_without_error();
}

#[test]
fn cross_file_hashmap_string_key_borrows() {
    let mut test = MultiFileTest::new();

    test.add_file(
        "anim_data.wj",
        r#"
pub struct AnimData {
    pub name: string,
    pub duration: f32,
}
"#,
    );

    test.add_file(
        "controller.wj",
        r#"
use std::collections::HashMap
use crate::anim_data::AnimData

pub struct Controller {
    pub animations: HashMap<string, AnimData>,
}

impl Controller {
    pub fn play(self, anim_name: string) {
        if let Some(animation) = self.animations.get(anim_name) {
            let d = animation.duration
        }
    }
}
"#,
    );

    test.assert_compiles_without_error();
}
