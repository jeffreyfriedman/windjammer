//! E0308 Phase 9: Verify Pattern B - Pattern binding deref (match on Index)
//!
//! match self.nodes[i] { BlendNode::Lerp { node_a, node_b } => BlendNode::Lerp { node_a, node_b } }
//! should emit *node_a, *node_b when struct expects u32

use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let input = temp_dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_bin)
        .args([
            "build",
            input.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    let rust_file = out_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).unwrap_or_else(|_| {
        panic!(
            "Generated .rs file not found at {:?}\nstdout: {}\nstderr: {}",
            rust_file,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
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
    // Codegen may produce *node_a or *(node_a) — both are valid deref
    let has_deref_a = rust.contains("*node_a") || rust.contains("*(node_a)");
    let has_deref_b = rust.contains("*node_b") || rust.contains("*(node_b)");
    assert!(
        has_deref_a && has_deref_b,
        "Expected *node_a and *node_b (or *(node_a) and *(node_b)) when using match-on-index bindings. Got:\n{}",
        rust
    );
}
