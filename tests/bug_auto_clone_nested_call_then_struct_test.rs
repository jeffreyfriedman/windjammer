#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! TDD: auto-clone when a local is moved into a nested call argument then reused (E0382).
//!
//! Mirrors `windjammer-game-core/.../uv_unwrap.wj`: `outer(a, b, inner(v))` moves `v`, then `v`
//! is used again (e.g. in a struct literal on the next statement).

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_clone_after_move_in_nested_call_then_struct_field() {
    let source = r#"
pub fn dup_v(v: Vec<usize>) -> Vec<usize> {
    v
}

pub fn merge_three(a: usize, b: Vec<usize>, c: Vec<usize>) -> Vec<usize> {
    c
}

pub struct Island {
    pub uvs: Vec<usize>,
}

pub fn unwrap_like_uv(positions_len: usize, indices: Vec<usize>) -> Vec<usize> {
    if positions_len == 0 {
        let empty: Vec<usize> = vec![]
        let z = Island { uvs: empty }
        return z.uvs
    }
    let mut corner_uv: Vec<usize> = vec![]
    let mut i: usize = 0
    while i < 3 {
        corner_uv.push(i)
        i = i + 1
    }

    let merged = merge_three(positions_len, indices, dup_v(corner_uv))
    let island = Island {
        uvs: corner_uv,
    }
    merged
}
"#;

    let rust = parse_and_generate(source);

    assert!(
        rust.contains("dup_v(") && rust.contains("corner_uv.clone()"),
        "Expected dup_v(corner_uv.clone()) pattern; generated:\n{}",
        rust
    );
}

/// Same pattern as `uv_unwrap.wj`: first argument is `positions.len()`, like
/// `per_vertex_average_uv(positions.len(), indices, duplicate_uv_coords(corner_uv))`.
#[test]
fn test_clone_after_move_with_len_in_outer_call_then_struct_field() {
    let source = r#"
pub fn dup_v(v: Vec<usize>) -> Vec<usize> {
    v
}

pub fn merge_three(a: usize, b: Vec<usize>, c: Vec<usize>) -> Vec<usize> {
    c
}

pub struct Island {
    pub uvs: Vec<usize>,
}

pub fn planar_like(positions: Vec<usize>, indices: Vec<usize>) -> Vec<usize> {
    if positions.len() == 0 {
        return vec![]
    }
    let mut corner_uv: Vec<usize> = vec![]
    let mut i: usize = 0
    while i < 3 {
        corner_uv.push(i)
        i = i + 1
    }

    let merged = merge_three(positions.len(), indices, dup_v(corner_uv))
    let island = Island {
        uvs: corner_uv,
    }
    merged
}
"#;

    let rust = parse_and_generate(source);

    assert!(
        rust.contains("dup_v(") && rust.contains("corner_uv.clone()"),
        "Expected dup_v(corner_uv.clone()) with positions.len(); generated:\n{}",
        rust
    );
}
