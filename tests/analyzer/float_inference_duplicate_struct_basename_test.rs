//! TDD: Duplicate struct basenames across modules (e.g. two `CharacterController` types).
//! Unqualified registry lookup returns None → `self.field` did not constrain float literals → E0308 (f32 vs f64).
//!
//! Fix: resolve `self` fields via `current_file_module_path::TypeName` first; push/pop path in nested `mod`.

use windjammer::parser::ast::types::Type;
use windjammer::*;

#[test]
fn test_self_field_float_disambiguated_by_module_path() {
    let source = r#"
mod alpha {
    pub struct Widget {
        pub rate: f32,
    }

    impl Widget {
        pub fn decay(self) {
            let _ = 1.0 - self.rate
        }
    }
}

mod beta {
    pub struct Widget {
        pub rate: f32,
    }

    impl Widget {
        pub fn decay(self) {
            let _ = 2.0 - self.rate
        }
    }
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut float_inference = type_inference::FloatInference::new();
    float_inference.infer_program(&program);

    assert!(
        float_inference.errors.is_empty(),
        "float inference errors: {:?}",
        float_inference.errors
    );

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, trait_methods) =
        analyzer.analyze_program(&program).expect("analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.set_analyzed_trait_methods(trait_methods);
    let rust = generator.generate_program(&program, &analyzed);

    assert!(
        rust.contains("1.0_f32") && rust.contains("2.0_f32"),
        "literals next to self.rate must be f32; got:\n{}",
        rust
    );
    assert!(
        !rust.contains("1.0_f64") && !rust.contains("2.0_f64"),
        "must not default to f64 when self field is f32; got:\n{}",
        rust
    );
}

#[test]
fn test_file_module_path_qualifies_impl_self_fields() {
    let source = r#"
pub struct DupName {
    pub v: f32,
}

impl DupName {
    pub fn mix(self) {
        let _ = 1.0 - self.v
    }
}
"#;

    let mut lexer = lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = parser::Parser::new(tokens);
    let program = parser.parse().expect("parse");

    let mut float_inference = type_inference::FloatInference::new();
    // Same layout as library compile: file `foo/bar.wj` → ["foo", "bar"]
    float_inference.set_current_file_module_path(vec!["foo".to_string(), "bar".to_string()]);

    // Another `DupName` elsewhere makes basename lookup ambiguous (dogfood pattern).
    let mut other_fields = std::collections::HashMap::new();
    other_fields.insert("v".to_string(), Type::Custom("f32".to_string()));
    let mut extra = std::collections::HashMap::new();
    extra.insert("other::DupName".to_string(), other_fields);
    float_inference.set_global_struct_field_types(&extra);

    float_inference.infer_program(&program);

    assert!(
        float_inference.errors.is_empty(),
        "float inference errors: {:?}",
        float_inference.errors
    );

    let mut analyzer = analyzer::Analyzer::new();
    let (analyzed, signatures, trait_methods) =
        analyzer.analyze_program(&program).expect("analyze");

    let mut generator = codegen::CodeGenerator::new(signatures, CompilationTarget::Rust);
    generator.set_float_inference(float_inference);
    generator.set_analyzed_trait_methods(trait_methods);
    let rust = generator.generate_program(&program, &analyzed);

    assert!(
        rust.contains("1.0_f32"),
        "expected qualified-module self field to pin f32 literal; got:\n{}",
        rust
    );
    assert!(
        !rust.contains("1.0_f64"),
        "must not emit f64 for f32 field op; got:\n{}",
        rust
    );
}
