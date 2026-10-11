//! Fuzz target for the lexer + parser.
//!
//! Invariants (the same ones `tests/lexer_parser_never_panic_test.rs` checks on seeded inputs):
//! - neither the lexer nor the parser ever panics, on any input;
//! - an `Invalid` token is never accepted silently;
//! - every parse error line carries `line N, column M`;
//! - parsing is deterministic.
//!
//! Run with: cargo fuzz run fuzz_parser

#![no_main]

use libfuzzer_sys::fuzz_target;
use windjammer::lexer::{Lexer, Token};
use windjammer::parser::Parser;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else { return };
    let tokens = Lexer::new(input).tokenize_with_locations();
    let had_invalid = tokens.iter().any(|t| matches!(t.token, Token::Invalid(_)));
    let first = Parser::new(tokens.clone()).parse().map(|_| ());
    if had_invalid {
        assert!(first.is_err(), "invalid token accepted");
    }
    if let Err(msg) = &first {
        for line in msg.lines().filter(|l| !l.trim().is_empty()) {
            assert!(line.contains("line ") && line.contains(", column "), "no position: {line}");
        }
    }
    let second = Parser::new(tokens).parse().map(|_| ());
    assert_eq!(first.err(), second.err(), "non-deterministic parse");
});
