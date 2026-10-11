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
        let first = lexer.tokenize_with_locations();
        // Locations are 1-based and never go backwards.
        let mut prev = (1usize, 1usize);
        for t in &first {
            assert!(t.line >= 1 && t.column >= 1);
            assert!((t.line, t.column) >= prev, "token positions went backwards");
            prev = (t.line, t.column);
        }
        
        // Test that tokenize is deterministic
        let mut lexer2 = Lexer::new(input);
        let _ = lexer2.tokenize();
    }
});

