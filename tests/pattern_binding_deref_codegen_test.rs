//! TDD: Pattern binding deref when matching on ref (Index returns &T)
//!
//! Bug: match self.nodes[i] { BlendNode::Lerp { node_a, node_b, .. } => BlendNode::Lerp { node_a, node_b, .. } }
//! generates node_a, node_b as &u32 but struct expects u32 → E0308 "expected u32, found &u32"
//!
//! Fix: When pattern bindings come from match-on-ref (Index, etc.), deref Copy types in struct literals.

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed)
}

/// Match on Vec index, use bindings in enum variant struct - should deref Copy types
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

    let rust = compile_and_get_rust(source);
    // Pattern bindings from match on &BlendNode should be dereferenced: *node_a, *node_b
    assert!(
        rust.contains("*node_a") && rust.contains("*node_b"),
        "Expected *node_a and *node_b when using match-on-index bindings in struct. Got:\n{}",
        rust
    );
}
