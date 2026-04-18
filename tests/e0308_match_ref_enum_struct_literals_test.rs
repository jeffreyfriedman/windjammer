//! TDD: E0308 when rebuilding enum variants from `if let` / `match` on `&vec[i]` (non-Copy element).
//!
//! Rust binds `BlendNode::Blend1D { clips, parameter }` as `&Vec<_>` and `&f32`; struct literals need
//! `clips.clone()` and `*(parameter)` (Copy peel).

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

#[test]
fn test_if_let_vec_index_enum_struct_copy_and_clone_fields() {
    let source = r#"
@derive(Clone)
pub struct Clip {
    pub t: f32,
}

pub enum Node {
    Blend1D { clips: Vec<Clip>, parameter: f32 },
}

pub struct Tree {
    pub nodes: Vec<Node>,
}

impl Tree {
    pub fn touch(self, id: u32, v: f32) {
        if (id as usize) < self.nodes.len() {
            if let Node::Blend1D { clips, parameter } = self.nodes[id as usize] {
                self.nodes[id as usize] = Node::Blend1D { clips, parameter: v }
            }
        }
    }

    pub fn push_clip(self, id: u32, c: Clip) {
        if (id as usize) < self.nodes.len() {
            if let Node::Blend1D { clips, parameter } = self.nodes[id as usize] {
                let mut new_clips = clips
                new_clips.push(c)
                self.nodes[id as usize] = Node::Blend1D { clips: new_clips, parameter }
            }
        }
    }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("clips.clone()") || rust.contains("new_clips = clips.clone()"),
        "non-Copy Vec binding should clone for owned use; got:\n{rust}"
    );
    assert!(
        rust.contains("*(parameter)") || rust.contains("*parameter"),
        "Copy f32 binding from &enum should deref in struct literal; got:\n{rust}"
    );
}
