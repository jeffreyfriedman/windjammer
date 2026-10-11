#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! Totality property: the lexer and parser never panic, whatever the input, and every failure
//! carries a line and column.
//!
//! Three input classes (all seeded and reproducible; a failing seed is printed):
//!   1. arbitrary text (random unicode scalars and ASCII punctuation),
//!   2. token soup (random sequences of language tokens: syntactically plausible, mostly invalid),
//!   3. mutated real programs (the conformance corpus with deleted/duplicated/inserted spans).
//! For `cargo-fuzz` continuous fuzzing see fuzz/fuzz_targets/fuzz_parser.rs, which asserts the
//! same invariants via `windjammer::parser::check_total`-style logic.

use std::panic::{catch_unwind, AssertUnwindSafe};
use windjammer::lexer::{Lexer, Token};
use windjammer::parser::Parser;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

const SOUP: &[&str] = &[
    "fn", "let", "mut", "const", "static", "struct", "enum", "trait", "impl", "match", "if", "else",
    "for", "in", "while", "loop", "return", "break", "continue", "use", "mod", "pub", "self", "as",
    "where", "type", "dyn", "async", "await", "defer", "thread", "unsafe", "extern", "int", "string",
    "x", "y", "foo", "Bar", "Some", "None", "Ok", "Err", "_", "true", "false", "0", "1", "42",
    "18446744073709551615", "99999999999999999999999999999999999999999", "0x", "0xFF", "0b", "0o17",
    "1.5", "1e", "1e9", "1..", "..", "..=", "'a'", "'\\n'", "'outer", "'outer:", "'", "\"s\"", "\"",
    "\"${x}\"", "\"${\"", "\"${\"a\".len()}\"", "\"${", "r#\"raw\"#", "r#", "@derive(Clone)", "@",
    "#", "#[derive(Debug)]", "$", "`", "\\", "~", "(", ")", "{", "}", "[", "]", ",", ";", ":", "::",
    ".", "?", "!", "+", "-", "*", "/", "%", "=", "==", "!=", "<", ">", "<=", ">=", "&", "&&", "|",
    "||", "|>", "|>?", "^", "<<", ">>", "+=", "-=", "*=", "->", "=>", "<-", "<T>", "::<int>",
    "// comment", "/// doc", "\n", "\n", "\n",
];

fn lex_and_parse(src: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_with_locations();
    let had_invalid = tokens.iter().any(|t| matches!(t.token, Token::Invalid(_)));
    let mut parser = Parser::new_with_source(tokens.clone(), "fuzz.wj".to_string(), src.to_string());
    let first = parser.parse().map(|_| ());
    // Invalid tokens must never be accepted silently.
    if had_invalid {
        assert!(first.is_err(), "Invalid token accepted without an error: {src:?}");
    }
    // Determinism.
    let second = Parser::new_with_source(tokens, "fuzz.wj".to_string(), src.to_string())
        .parse()
        .map(|_| ());
    assert_eq!(first.as_ref().err(), second.as_ref().err(), "non-deterministic: {src:?}");
    if let Err(msg) = &first {
        for line in msg.lines().filter(|l| !l.trim().is_empty()) {
            assert!(
                line.contains("line ") && line.contains(", column "),
                "error without line/column: {line:?}\nfor input {src:?}"
            );
        }
    }
    first
}

fn check(src: &str, seed: u64) {
    let owned = src.to_string();
    let r = catch_unwind(AssertUnwindSafe(|| lex_and_parse(&owned)));
    if let Err(p) = r {
        let msg = p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        panic!("seed {seed}: lexer/parser violated totality: {msg}\ninput: {src:?}");
    }
}

fn big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e));
}

#[test]
fn arbitrary_text_never_panics() {
    big_stack(|| {
        let mut rng = Rng(0xDEADBEEFCAFEF00D);
        for case in 0..4000u64 {
            let len = rng.below(120);
            let mut s = String::new();
            for _ in 0..len {
                let c = match rng.below(4) {
                    0 => char::from_u32(rng.below(0x2FFF) as u32).unwrap_or('?'),
                    1 => char::from_u32(32 + rng.below(95) as u32).unwrap(),
                    2 => ['#', '$', '`', '\'', '"', '\\', '{', '}', '@', '~', '\n'][rng.below(11)],
                    _ => char::from_u32(rng.below(0x110000) as u32).unwrap_or('\u{FFFD}'),
                };
                s.push(c);
            }
            check(&s, case);
        }
    });
}

#[test]
fn token_soup_never_panics() {
    big_stack(|| {
        let mut rng = Rng(0x0123456789ABCDEF);
        for case in 0..6000u64 {
            let n = 1 + rng.below(40);
            let mut s = String::new();
            for _ in 0..n {
                s.push_str(SOUP[rng.below(SOUP.len())]);
                s.push(if rng.below(4) == 0 { '\n' } else { ' ' });
            }
            check(&s, case);
        }
    });
}

fn corpus() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance_programs");
    let mut out = Vec::new();
    let mut files: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wj"))
        .collect();
    files.sort();
    for f in files {
        if let Ok(s) = std::fs::read_to_string(f) {
            out.push(s);
        }
    }
    out
}

#[test]
fn mutated_programs_never_panic() {
    big_stack(|| {
        let programs = corpus();
        assert!(programs.len() > 20, "corpus missing");
        let mut rng = Rng(0xFEEDFACE12345678);
        for case in 0..3000u64 {
            let base: Vec<char> = programs[rng.below(programs.len())].chars().collect();
            let mut s = base.clone();
            for _ in 0..1 + rng.below(4) {
                if s.is_empty() {
                    break;
                }
                let a = rng.below(s.len());
                let b = (a + 1 + rng.below(12)).min(s.len());
                match rng.below(5) {
                    0 => {
                        s.drain(a..b);
                    }
                    1 => {
                        let dup: Vec<char> = s[a..b].to_vec();
                        for (i, c) in dup.into_iter().enumerate() {
                            s.insert(a + i, c);
                        }
                    }
                    2 => {
                        let t: Vec<char> = SOUP[rng.below(SOUP.len())].chars().collect();
                        for (i, c) in t.into_iter().enumerate() {
                            s.insert(a + i, c);
                        }
                    }
                    3 => s.truncate(a),
                    _ => {
                        let j = rng.below(s.len());
                        s.swap(a, j);
                    }
                }
            }
            let text: String = s.into_iter().collect();
            check(&text, case);
        }
    });
}

#[test]
fn every_corpus_program_parses_without_error() {
    // Anchor: the mutation test is only meaningful if the unmutated programs are valid.
    big_stack(|| {
        let mut failures = Vec::new();
        for (i, src) in corpus().iter().enumerate() {
            if let Err(e) = lex_and_parse(src) {
                failures.push(format!("program #{i}: {e}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}

#[test]
fn unexpected_characters_are_positioned_diagnostics() {
    for (src, line, col, needle) in [
        ("#[derive(Clone)]\nstruct S {}\n", 1, 1, "@derive(Clone)"),
        ("fn main() {\n    let x = 1 ~ 2\n}\n", 2, 15, "unexpected character '~'"),
        ("fn main() {\n    let x = $y\n}\n", 2, 13, "'$'"),
        ("\n\n  `x`", 3, 3, "unexpected character"),
    ] {
        let err = lex_and_parse(src).expect_err(src);
        let expect = format!("(at line {line}, column {col})");
        assert!(err.contains(&expect), "{src:?}: want {expect} in {err}");
        assert!(err.contains(needle), "{src:?}: want {needle:?} in {err}");
    }
}

#[test]
fn several_errors_are_reported_per_file() {
    let src = "fn a() {\n    let = 1\n}\nfn b() {\n    let y = +\n}\nfn c() { 1 }\nstruct { }\n";
    let err = lex_and_parse(src).expect_err("bad program");
    assert!(err.lines().count() >= 3, "expected >= 3 errors, got:\n{err}");
    for l in [2, 5, 8] {
        assert!(err.contains(&format!("line {l},")), "no error on line {l}:\n{err}");
    }
}
