//! E0308 Phase 9: Verify Pattern B - Pattern binding deref (match on Index)
//!
//! match self.nodes[i] { BlendNode::Lerp { node_a, node_b } => BlendNode::Lerp { node_a, node_b } }
//! should emit *node_a, *node_b when struct expects u32

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let test_dir = std::env::temp_dir().join("wj_e0308_pattern_b");
    let _ = std::fs::create_dir_all(&test_dir);
    let input = test_dir.join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    let output = Command::new(&wj_bin)
        .arg("build")
        .arg(&input)
        .current_dir(&test_dir)
        .output()
        .expect("wj build");

    let rust_file = test_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).unwrap_or_else(|_| {
        String::from_utf8_lossy(&output.stderr).to_string()
    })
}

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

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("*node_a") && rust.contains("*node_b"),
        "Expected *node_a and *node_b when using match-on-index bindings. Got:\n{}",
        rust
    );
}
