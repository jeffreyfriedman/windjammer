//! TDD: Cross-file ownership via signature registry (same as loading from `.wj.meta` at compile time).
//! When file A defines a function with a borrowed parameter, file B's codegen should insert `&`
//! at the call site when that signature is merged before analysis.

use windjammer::analyzer::{Analyzer, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn compile_with_external_sigs(source: &str, external_sigs: &SignatureRegistry) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_fns, registry, _) = analyzer
        .analyze_program_with_global_signatures(&program, external_sigs)
        .unwrap();
    let mut codegen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    codegen.generate_program(&program, &analyzed_fns)
}

#[test]
fn test_cross_file_borrowed_param_gets_ampersand() {
    // File A: defines a function with a read-only Vec parameter (inferred as borrowed for Rust)
    let file_a_source = r#"
struct AABB {
    min_x: f32,
    max_x: f32,
}

fn check_collisions(walls: Vec<AABB>) -> bool {
    let mut i = 0
    while i < walls.len() {
        if walls[i].min_x > 0.0 {
            return true
        }
        i = i + 1
    }
    false
}
"#;

    let mut lexer_a = Lexer::new(file_a_source);
    let tokens_a = lexer_a.tokenize_with_locations();
    let parser_a = Box::leak(Box::new(Parser::new(tokens_a)));
    let program_a = parser_a.parse().unwrap();
    let mut analyzer_a = Analyzer::new();
    let (_, registry_a, _) = analyzer_a.analyze_program(&program_a).unwrap();

    let sig = registry_a.get_signature("check_collisions").unwrap();
    assert_eq!(
        sig.param_ownership[0],
        OwnershipMode::Borrowed,
        "walls should be inferred as Borrowed"
    );

    // File B: calls check_collisions only (callee lives in external registry)
    let file_b_source = r#"
struct AABB {
    min_x: f32,
    max_x: f32,
}

fn get_walls() -> Vec<AABB> {
    Vec::new()
}

fn game_update() {
    let walls = get_walls()
    let result = check_collisions(walls)
}
"#;

    let code = compile_with_external_sigs(file_b_source, &registry_a);

    assert!(
        code.contains("check_collisions(&walls)"),
        "Cross-file call should insert & for borrowed parameter. Got:\n{}",
        code
    );
}
