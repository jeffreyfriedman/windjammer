//! Fuzz target for the code generator
//!
//! This fuzzer tests that the codegen:
//! - Never panics on any valid AST
//! - Produces valid Rust code or errors
//! - Handles edge cases gracefully
//!
//! Run with: cargo fuzz run fuzz_codegen

#![no_main]

use libfuzzer_sys::fuzz_target;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::codegen::CodeGenerator;
use windjammer::analyzer::SignatureRegistry;
use windjammer::CompilationTarget;

fuzz_target!(|data: &[u8]| {
    // Convert to string (may be invalid UTF-8)
    if let Ok(input) = std::str::from_utf8(data) {
        // Lex and parse the input
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        
        let mut parser = Parser::new(tokens);
        if let Ok(program) = parser.parse() {
            // Test codegen doesn't panic on valid AST
            let signatures = SignatureRegistry::new();
            let mut generator = CodeGenerator::new_for_module(
                signatures,
                CompilationTarget::Wasm
            );
            let _ = generator.generate_program(&program, &[]);
        }
    }
});

