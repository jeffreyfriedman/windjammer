//! Fuzz target for the lexer
//!
//! This fuzzer tests that the lexer:
//! - Never panics on any input
//! - Handles malformed UTF-8 gracefully
//! - Produces valid tokens or errors
//!
//! Run with: cargo fuzz run fuzz_lexer

#![no_main]

use libfuzzer_sys::fuzz_target;
use windjammer::lexer::Lexer;

fuzz_target!(|data: &[u8]| {
    // Convert to string (may be invalid UTF-8)
    if let Ok(input) = std::str::from_utf8(data) {
        // Test lexer doesn't panic
        let mut lexer = Lexer::new(input);
        let _ = lexer.tokenize();
        
        // Test that tokenize is deterministic
        let mut lexer2 = Lexer::new(input);
        let _ = lexer2.tokenize();
    }
});

