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

//! P3.647b: product `BlendTree::evaluate_node(clips: Vec<_>)` keeps Owned emission while
//! `forwarding_borrow_params[clips]=true` (forwarded into demoted `sample_clip_pose(&Vec)`).
//! IR must not emit `&clips` into that owned formal.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Clip { pub id: u32 }
pub struct Pose { pub v: u32 }

pub enum Node {
    Leaf { animation_id: u64 },
    Blend { node_a: u32, node_b: u32, weight: f32 },
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: Option<u32>,
}

impl Tree {
    fn evaluate_node(self, node_id: u32, clips: Vec<Clip>, time: f32) -> Pose {
        match self.nodes[node_id as usize] {
            Node::Leaf { animation_id } => {
                sample_clip_pose(clips, animation_id, time)
            },
            Node::Blend { node_a, node_b, weight } => {
                let pose_a = self.evaluate_node(node_a, clips, time)
                let pose_b = self.evaluate_node(node_b, clips, time)
                Pose { v: pose_a.v + pose_b.v + (weight as u32) }
            },
        }
    }
}

fn sample_clip_pose(clips: Vec<Clip>, animation_id: u64, time: f32) -> Pose {
    if (animation_id as usize) >= clips.len() {
        return Pose { v: 0 }
    }
    Pose { v: clips[animation_id as usize].id + (time as u32) }
}
"#;

#[test]
fn owned_vec_formal_with_forwarding_borrow_must_not_reborrow() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("emit:\n{rs}");
    assert!(
        rs.contains("clips: Vec<Clip>"),
        "expected owned Vec formal:\n{rs}"
    );
    assert!(
        rs.contains("sample_clip_pose(&clips") || rs.contains("sample_clip_pose(clips"),
        "expected sample_clip_pose call:\n{rs}"
    );
    assert!(
        !rs.contains(", &clips,") && !rs.contains(", &clips)"),
        "owned Vec formal must not receive &clips:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
