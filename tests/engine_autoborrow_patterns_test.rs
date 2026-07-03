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

//! TDD: Auto-borrow patterns from windjammer-game-core dogfooding.
//!
//! These tests capture real compilation failures in the game engine that
//! stem from incorrect auto-borrow decisions in call_site_borrow.rs.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn copy_struct_param_not_over_borrowed() {
    // Pattern: find_node_index(id: NodeId) where NodeId is Copy
    // Bug: codegen adds & at call site → &NodeId where NodeId expected
    let source = r#"
pub struct NodeId {
    pub value: int,
}

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

    pub fn remove(self, id: NodeId) {
        let idx = self.find_node_index(id)
    }

    pub fn has_node(self, id: NodeId) -> bool {
        self.find_node_index(id) >= 0
    }
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Copy struct param should not get & at call site.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("&parent") && !generated.contains("&id"),
        "NodeId is Copy — no & at call site.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn copy_vec3_param_not_over_borrowed() {
    // Pattern: is_in_sight_cone(npc_pos, npc_forward, target_pos)
    // Bug: codegen adds & to Vec3 param → &Vec3 where Vec3 expected
    let source = r#"
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Perception {
    pub range: f32,
}

impl Perception {
    pub fn is_in_sight_cone(self, origin: Vec3, forward: Vec3, target: Vec3) -> bool {
        let dx = target.x - origin.x
        dx > 0.0
    }

    pub fn check_target(self, npc_pos: Vec3, npc_forward: Vec3, target_pos: Vec3) {
        let result = self.is_in_sight_cone(npc_pos, npc_forward, target_pos)
    }
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Copy Vec3 param should not get & at call site.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn hashmap_get_with_string_key_auto_borrows() {
    // Pattern: self.animations.get(anim_name)
    // Bug: codegen passes owned String; HashMap::get expects &K
    let source = r#"
use std::collections::HashMap

pub struct AnimData {
    pub name: string,
    pub duration: f32,
}

pub struct Controller {
    pub animations: HashMap<string, AnimData>,
    pub current: string,
}

impl Controller {
    pub fn play(self, anim_name: string) {
        if let Some(animation) = self.animations.get(anim_name) {
            let d = animation.duration
        }
    }
}
"#;

    let (generated, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "HashMap.get(key) should auto-borrow key to &key.\nGenerated:\n{}",
        generated
    );
}
