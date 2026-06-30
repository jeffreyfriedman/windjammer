#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Pattern binding deref when matching on ref (Index returns &T)
//!
//! Bug: match self.nodes[i] { BlendNode::Lerp { node_a, node_b, .. } => BlendNode::Lerp { node_a, node_b, .. } }
//! generates node_a, node_b as &u32 but struct expects u32 → E0308 "expected u32, found &u32"
//!
//! Fix: When pattern bindings come from match-on-ref (Index, etc.), deref Copy types in struct literals.

// Match on Vec index, use bindings in enum variant struct - should deref Copy types

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_index_bindings_deref_in_struct() {
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
    // Two valid strategies: (1) match on ref with *(node_a) deref, or
    // (2) clone borrow-break with owned bindings. Either is correct Rust.
    let uses_deref = rust.contains("*(node_a)") && rust.contains("*(node_b)");
    let uses_clone_break = rust.contains("__match_borrow_break")
        && rust.contains("BlendNode::Lerp { node_a, node_b,");
    assert!(
        uses_deref || uses_clone_break,
        "Expected deref *(node_a) or clone borrow-break for match-on-index. Got:\n{}",
        rust
    );
}
