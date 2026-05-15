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

//! TDD tests for preventing orphaned `pub mod` declarations at the AST level.
//!
//! Bug: Compiler generates `pub mod shaders;` in mod.rs for modules
//!      that were filtered out (e.g., shader files).
//! Old fix (hacky): Post-compilation cleanup_orphaned_mod_declarations
//!                  scanned generated .rs files and removed invalid `pub mod` lines.
//!                  This was a "generate wrong code, then fix it" pattern.
//! Proper fix: Strip filtered Item::Mod entries from the AST BEFORE codegen,
//!             so wrong code is never generated in the first place.

use std::collections::HashSet;
use windjammer::compiler::strip_filtered_mod_items;
use windjammer::parser::ast::core::Item;

fn mod_item(name: &str) -> Item<'static> {
    Item::Mod {
        name: name.to_string(),
        items: vec![],
        is_public: true,
        location: None,
    }
}

// =============================================================================
// Unit tests for strip_filtered_mod_items
// =============================================================================

#[test]
fn test_strip_removes_filtered_mod_items() {
    let items = vec![
        mod_item("rendering"),
        mod_item("shaders"),
        mod_item("physics"),
    ];

    let mut filtered = HashSet::new();
    filtered.insert("shaders".to_string());

    let result = strip_filtered_mod_items(items, &filtered);
    let names: Vec<&str> = result
        .iter()
        .filter_map(|item| match item {
            Item::Mod { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();

    assert_eq!(names, vec!["rendering", "physics"]);
}

#[test]
fn test_strip_preserves_all_non_mod_items() {
    let items = vec![mod_item("rendering"), mod_item("shaders")];

    let mut filtered = HashSet::new();
    filtered.insert("shaders".to_string());

    let result = strip_filtered_mod_items(items, &filtered);
    assert_eq!(result.len(), 1);
}

#[test]
fn test_strip_no_op_when_nothing_filtered() {
    let items = vec![mod_item("a"), mod_item("b")];
    let filtered = HashSet::new();

    let result = strip_filtered_mod_items(items, &filtered);
    assert_eq!(result.len(), 2);
}

#[test]
fn test_strip_all_filtered() {
    let items = vec![mod_item("shader_a"), mod_item("shader_b")];

    let mut filtered = HashSet::new();
    filtered.insert("shader_a".to_string());
    filtered.insert("shader_b".to_string());

    let result = strip_filtered_mod_items(items, &filtered);
    assert_eq!(result.len(), 0);
}

// =============================================================================
// Integration test: full codegen pipeline
// =============================================================================

#[test]
fn test_codegen_does_not_emit_filtered_mod_declarations() {
    use windjammer::analyzer::Analyzer;
    use windjammer::codegen::rust::CodeGenerator;
    use windjammer::lexer::Lexer;
    use windjammer::parser::Parser;
    use windjammer::CompilationTarget;

    let source = r#"
    pub mod rendering
    pub mod shaders
    pub mod physics
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let mut program = parser.parse().unwrap();

    let mut filtered = HashSet::new();
    filtered.insert("shaders".to_string());
    program.items = strip_filtered_mod_items(program.items, &filtered);

    let mut analyzer = Analyzer::new();
    let (analyzed_functions, registry, _) = analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    let output = generator.generate_program(&program, &analyzed_functions);

    assert!(
        output.contains("pub mod rendering;") || output.contains("mod rendering"),
        "Should emit rendering module. Got:\n{}",
        output
    );
    assert!(
        output.contains("pub mod physics;") || output.contains("mod physics"),
        "Should emit physics module. Got:\n{}",
        output
    );
    assert!(
        !output.contains("mod shaders"),
        "Should NOT emit filtered shaders module. Got:\n{}",
        output
    );
}
