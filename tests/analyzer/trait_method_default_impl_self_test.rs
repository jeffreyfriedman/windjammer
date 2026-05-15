//! TDD: Trait methods with default implementations should infer &self
//!
//! BUG: Trait methods infer `self` (owned) causing E0277
//!
//! Example:
//! ```windjammer
//! trait GameLoop {
//!     fn init(self) {
//!         // Default implementation
//!     }
//! }
//! ```
//!
//! GENERATED (WRONG):
//! ```rust
//! trait GameLoop {
//!     fn init(self) {  // ❌ E0277: Self not Sized
//!     }
//! }
//! ```
//!
//! EXPECTED: Should infer &self or &mut self

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};
use windjammer::CompilationTarget;

fn parse_code(code: &str) -> Program<'static> {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().unwrap()
}

#[test]
fn test_trait_method_default_impl_infers_ref_self() {
    let source = r#"
        trait GameLoop {
            fn init(self) {
                // Default implementation
            }
            
            fn update(self, delta: f32) {
                // Default implementation
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);

    println!("Generated Rust:\n{}", rust_code);

    // Trait methods with default implementations should use &self
    assert!(
        rust_code.contains("fn init(&self)") || rust_code.contains("fn init(&mut self)"),
        "Trait method with default impl should infer &self or &mut self, got:\n{}",
        rust_code
    );
}

#[test]
fn test_trait_method_mutating_default_impl() {
    let source = r#"
        trait Counter {
            fn increment(self) {
                // If this modifies state, needs &mut self
            }
        }
    "#;

    let program = parse_code(source);
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &analyzed_functions);

    println!("Generated Rust:\n{}", rust_code);

    // Should infer &self or &mut self
    assert!(
        rust_code.contains("fn increment(&self)") || rust_code.contains("fn increment(&mut self)"),
        "Trait method with default impl should infer reference, got:\n{}",
        rust_code
    );
}
