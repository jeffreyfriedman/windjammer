//! E0308 Phase 9: Verify Pattern B - Pattern binding deref (match on Index)
//!
//! match self.nodes[i] { BlendNode::Lerp { node_a, node_b } => BlendNode::Lerp { node_a, node_b } }
//! should emit *node_a, *node_b when struct expects u32

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_on_index_bindings_get_deref() {
    let source = r#"
pub enum BlendNode {
    Lerp { node_a: u32, node_b: u32, blend_factor: f32 },
    Clip { id: u64 },
}

pub struct BlendTree {
    nodes: Vec<BlendNode>,
}

impl BlendTree {
    pub fn update_node(self, node_id: u32, value: f32) {
        if (node_id as usize) < self.nodes.len() {
            match self.nodes[node_id as usize] {
                BlendNode::Lerp { node_a, node_b, blend_factor } => {
                    self.nodes[node_id as usize] = BlendNode::Lerp { node_a, node_b, blend_factor: value }
                }
                _ => {}
            }
        }
    }
}
"#;

    let rust = test_utils::compile_single(source);
    // Codegen may produce *node_a or *(node_a) — both are valid deref
    let has_deref_a = rust.contains("*node_a") || rust.contains("*(node_a)");
    let has_deref_b = rust.contains("*node_b") || rust.contains("*(node_b)");
    assert!(
        has_deref_a && has_deref_b,
        "Expected *node_a and *node_b (or *(node_a) and *(node_b)) when using match-on-index bindings. Got:\n{}",
        rust
    );
}
