//! TDD tests for the decorator registry system.
//!
//! The decorator registry centralizes all knowledge about which decorators
//! are valid for which backends, replacing hardcoded matches! checks scattered
//! across codegen files.
//!
//! Bug: Decorator filtering was hardcoded in 4+ places with identical triplets
//!      ("vertex" | "fragment" | "compute"), violating DRY and making it easy
//!      to miss a new GPU-only decorator.
//! Root Cause: No centralized decorator metadata system.
//! Fix: DecoratorRegistry with backend-aware classification.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

// =============================================================================
// Test: DecoratorRegistry exists and classifies decorators
// =============================================================================

#[test]
fn test_decorator_registry_gpu_decorators_not_valid_for_rust() {
    use windjammer::decorator_registry::DecoratorRegistry;

    let registry = DecoratorRegistry::new();
    assert!(!registry.is_valid_for_backend("vertex", CompilationTarget::Rust));
    assert!(!registry.is_valid_for_backend("fragment", CompilationTarget::Rust));
    assert!(!registry.is_valid_for_backend("compute", CompilationTarget::Rust));
}

#[test]
fn test_decorator_registry_gpu_decorators_valid_for_wgsl() {
    use windjammer::decorator_registry::DecoratorRegistry;

    let registry = DecoratorRegistry::new();
    assert!(registry.is_gpu_decorator("vertex"));
    assert!(registry.is_gpu_decorator("fragment"));
    assert!(registry.is_gpu_decorator("compute"));
}

#[test]
fn test_decorator_registry_universal_decorators_valid_for_all() {
    use windjammer::decorator_registry::DecoratorRegistry;

    let registry = DecoratorRegistry::new();
    assert!(registry.is_valid_for_backend("test", CompilationTarget::Rust));
    assert!(registry.is_valid_for_backend("test", CompilationTarget::Go));
    assert!(registry.is_valid_for_backend("test", CompilationTarget::Wasm));
}

#[test]
fn test_decorator_registry_framework_decorators_classified() {
    use windjammer::decorator_registry::DecoratorRegistry;

    let registry = DecoratorRegistry::new();
    assert!(registry.is_framework_decorator("component"));
    assert!(registry.is_framework_decorator("game"));
    assert!(registry.is_framework_decorator("init"));
    assert!(registry.is_framework_decorator("update"));
    assert!(registry.is_framework_decorator("render"));
}

#[test]
fn test_decorator_registry_export_only_for_wasm() {
    use windjammer::decorator_registry::DecoratorRegistry;

    let registry = DecoratorRegistry::new();
    assert!(registry.is_valid_for_backend("export", CompilationTarget::Wasm));
    assert!(!registry.is_valid_for_backend("export", CompilationTarget::Rust));
    assert!(!registry.is_valid_for_backend("export", CompilationTarget::Go));
}

#[test]
fn test_decorator_registry_is_shader_file_uses_registry() {
    use windjammer::decorator_registry::DecoratorRegistry;

    let registry = DecoratorRegistry::new();
    let gpu_decorators = registry.gpu_decorator_names();
    assert!(gpu_decorators.contains(&"vertex"));
    assert!(gpu_decorators.contains(&"fragment"));
    assert!(gpu_decorators.contains(&"compute"));
}

// =============================================================================
// Test: GPU decorators are NOT emitted in Rust codegen
// =============================================================================

#[test]
fn test_vertex_decorator_not_emitted_in_rust() {
    let source = r#"
    @vertex
    fn my_vertex_shader() -> i32 {
        return 0
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        !output.contains("#[vertex]"),
        "GPU @vertex decorator should not appear in Rust output"
    );
    assert!(
        !output.contains("@vertex"),
        "GPU @vertex decorator should not appear in Rust output"
    );
}

#[test]
fn test_fragment_decorator_not_emitted_in_rust() {
    let source = r#"
    @fragment
    fn my_fragment_shader() -> i32 {
        return 0
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        !output.contains("#[fragment]"),
        "GPU @fragment decorator should not appear in Rust output"
    );
}

#[test]
fn test_compute_decorator_not_emitted_in_rust() {
    let source = r#"
    @compute
    fn my_compute_shader() -> i32 {
        return 0
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        !output.contains("#[compute]"),
        "GPU @compute decorator should not appear in Rust output"
    );
}

// =============================================================================
// Test: Framework decorators are NOT emitted in Rust codegen
// =============================================================================

#[test]
fn test_framework_decorators_not_emitted_in_rust() {
    let source = r#"
    @component
    struct Player {
        health: i32
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        !output.contains("#[component]"),
        "Framework @component decorator should not appear in Rust output"
    );
}

// =============================================================================
// Test: Unknown decorators still emit as attributes (extensibility)
// =============================================================================

#[test]
fn test_unknown_decorator_still_emitted() {
    let source = r#"
    @custom_attr
    fn my_fn() -> i32 {
        return 0
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("#[custom_attr]"),
        "Unknown decorators should pass through as Rust attributes: {}",
        output
    );
}
