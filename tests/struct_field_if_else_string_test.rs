//! TDD: String literals inside if-else expressions used as struct field values
//! must get `.to_string()` when the field type is String.
//!
//! Bug: `Property { value: if cond { "true" } else { "false" } }` generates
//! bare `"true"` / `"false"` instead of `"true".to_string()` / `"false".to_string()`.
//!
//! Root cause: Struct literal codegen only auto-converts direct Expression::Literal
//! to `.to_string()`, but doesn't propagate coercion into if-else branch bodies.

use windjammer::*;

fn compile_and_get_rust(source: &str) -> String {
    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse");

    let mut int_inference = type_inference::IntInference::new();
    int_inference.infer_program(&program);

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, _signatures, _trait_methods) = analyzer
        .analyze_program(&program)
        .expect("Failed to analyze");

    let registry = analyzer::SignatureRegistry::new();
    let mut generator = codegen::CodeGenerator::new(registry, CompilationTarget::Rust);
    generator.set_int_inference(int_inference);
    generator.set_float_inference(float_inference);
    generator.generate_program(&program, &analyzed)
}

#[test]
fn test_if_else_string_literal_in_struct_field() {
    let source = r#"
pub struct Property {
    pub name: string,
    pub value: string
}

pub fn boolean_prop(name: string, value: bool) -> Property {
    Property {
        name,
        value: if value { "true" } else { "false" },
    }
}
"#;
    let rust = compile_and_get_rust(source);

    assert!(
        rust.contains(r#""true".to_string()"#),
        "Expected \"true\".to_string() in if-else struct field. Got:\n{}",
        rust
    );
    assert!(
        rust.contains(r#""false".to_string()"#),
        "Expected \"false\".to_string() in if-else struct field. Got:\n{}",
        rust
    );
}

#[test]
fn test_if_else_empty_string_in_struct_field() {
    let source = r#"
pub struct Item {
    pub label: string
}

pub fn make_item(cond: bool) -> Item {
    Item {
        label: if cond { "hello" } else { "" },
    }
}
"#;
    let rust = compile_and_get_rust(source);

    assert!(
        rust.contains(r#""hello".to_string()"#),
        "Expected \"hello\".to_string(). Got:\n{}",
        rust
    );
    assert!(
        rust.contains(r#""".to_string()"#),
        "Expected \"\".to_string(). Got:\n{}",
        rust
    );
}
