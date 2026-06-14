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

//! E0308: `expected u32, found &u32` (and similar) from spurious references on Copy scalars.
//!
//! Root cause: match / Vec index on non-Copy yields `&T` bindings; `.clone()` on `&Copy` clones the
//! reference, not the value. Fix: infer enum pattern field types, track `&T` in `local_var_types`,
//! emit `*` for Copy pointees; dedupe `match &&expr` when index already borrowed.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_match_vec_index_enum_struct_copy_fields_use_star_not_ref_clone() {
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
    // Codegen uses `*(node_a)` so the deref applies to the binding, not a longer chain.
    assert!(
        rust.contains("*(node_a)") && rust.contains("*(node_b)"),
        "expected *(node_a) and *(node_b) for Copy fields; got:\n{rust}"
    );
    assert!(
        !rust.contains("match &&"),
        "must not emit match && (double borrow); got:\n{rust}"
    );
}

#[test]
fn test_vec_push_copy_scalar_strips_index_borrow() {
    let source = r#"
pub fn push_u32_from_vec(mask: Vec<u32>) {
    let mut acc = Vec::new()
    acc.push(mask[0 as usize])
}
"#;

    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("acc.push(mask[0_usize])") || rust.contains("acc.push(mask[0 as usize])"),
        "expected push without leading & on Copy vec index; got:\n{rust}"
    );
    assert!(
        !rust.contains("acc.push(&mask"),
        "must not pass &T to push when T: Copy; got:\n{rust}"
    );
}

#[test]
fn test_no_node_a_clone_after_match_on_vec_index() {
    let source = r#"
pub enum E { V { x: u32 } }
pub struct S { items: Vec<E> }
impl S {
    pub fn m(self) {
        match self.items[0 as usize] {
            E::V { x } => { let _ = x }
            _ => {}
        }
    }
}
"#;

    let rust = test_utils::compile_single(source);
    assert!(
        !rust.contains("(x).clone()"),
        "Copy binding from match should not use .clone(); got:\n{rust}"
    );
}
