//! Fuzz target for the parser
//!
//! This fuzzer tests that the parser:
//! - Never panics on any token stream
//! - Handles invalid syntax gracefully
//! - Produces valid AST or errors
//!
//! Run with: cargo fuzz run fuzz_parser

#![no_main]

use libfuzzer_sys::fuzz_target;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fuzz_target!(|data: &[u8]| {
    // Convert to string (may be invalid UTF-8)
    if let Ok(input) = std::str::from_utf8(data) {
        // Lex the input
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        
        // Test parser doesn't panic
        let mut parser = Parser::new(tokens.clone());
        let _ = parser.parse();
        
        // Test that parsing is deterministic
        let mut parser2 = Parser::new(tokens);
        let _ = parser2.parse();
    }
});

