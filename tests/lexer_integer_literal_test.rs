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

//! Integer literal typing (lexer): class-level property, not examples.
//!
//! Oracle: Rust's own integer parsing (`u128::from_str_radix` / `u64`), independent of the
//! lexer's code path. For every value and every radix the token must carry the same numeric
//! value, with the type fixed by the rule in `src/lexer/number_lexing.rs::int_token`.

use windjammer::lexer::{Lexer, Token};

fn lex1(src: &str) -> Token {
    let toks = Lexer::new(src).tokenize();
    assert_eq!(toks.len(), 2, "expected one token + Eof for {src:?}, got {toks:?}");
    toks[0].clone()
}

/// Reinterpret a token as (value as u128, Some(suffix)) following the documented payload rule.
fn decode(t: &Token) -> Option<(u128, Option<String>)> {
    match t {
        Token::IntLiteral(n) => Some((*n as u128 & u64::MAX as u128, None)),
        Token::IntLiteralSuffixed(n, s) => Some((*n as u64 as u128, Some(s.clone()))),
        _ => None,
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn spellings(v: u64) -> Vec<String> {
    vec![
        format!("{v}"),
        format!("0x{v:x}"),
        format!("0X{v:X}"),
        format!("0b{v:b}"),
        format!("0o{v:o}"),
        // underscore separators must not change the value
        format!("0x{:x}_{:08x}", v >> 32, v & 0xffff_ffff),
    ]
}

#[test]
fn every_u64_value_in_every_radix_keeps_its_value_and_gets_the_right_type() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut values: Vec<u64> = vec![
        0,
        1,
        i64::MAX as u64 - 1,
        i64::MAX as u64,
        i64::MAX as u64 + 1,
        i64::MAX as u64 + 2,
        u64::MAX - 1,
        u64::MAX,
        14695981039346656037, // FNV-1a 64 offset basis
        1099511628211,        // FNV-1a 64 prime
        0x9E37_79B9_7F4A_7C15,
    ];
    for _ in 0..2000 {
        let r = rng.next();
        values.push(r >> (rng.next() % 64));
    }
    for v in values {
        for src in spellings(v) {
            let tok = lex1(&src);
            let (got, suffix) = decode(&tok).unwrap_or_else(|| panic!("{src}: not an int token: {tok:?}"));
            assert_eq!(got, v as u128, "{src}: value changed ({tok:?})");
            if v <= i64::MAX as u64 {
                assert_eq!(tok, Token::IntLiteral(v as i64), "{src}");
            } else if v == 1 << 63 {
                assert_eq!(tok, Token::IntLiteral(i64::MIN), "{src}: 2^63 is the i64::MIN bit pattern");
            } else {
                assert_eq!(suffix.as_deref(), Some("u64"), "{src}: unsuffixed > i64::MAX must be u64");
            }
        }
    }
}

#[test]
fn explicit_suffixes_carry_bit_patterns() {
    for (src, bits, suf) in [
        ("18446744073709551615u64", u64::MAX, "u64"),
        ("0xFFFFFFFFFFFFFFFFu64", u64::MAX, "u64"),
        ("14695981039346656037u64", 14695981039346656037, "u64"),
        ("255u8", 255, "u8"),
        ("0xFFu8", 255, "u8"),
        ("0b1u32", 1, "u32"),
        ("18446744073709551615usize", u64::MAX, "usize"),
    ] {
        let (v, s) = decode(&lex1(src)).unwrap();
        assert_eq!((v as u64, s.as_deref()), (bits, Some(suf)), "{src}");
    }
}

#[test]
fn too_wide_or_malformed_literals_are_diagnostics_not_panics() {
    for src in [
        "18446744073709551616",                           // u64::MAX + 1
        "340282366920938463463374607431768211455",         // u128::MAX
        "340282366920938463463374607431768211456",         // u128::MAX + 1
        "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",            // 33 hex digits
        "0x",
        "0b",
        "0o",
        "0x_",
        "0b2",
        "9223372036854775808i64",                          // 2^63 does not fit i64
        "1e",
        "1e+",
        "99999999999999999999999999999999999999999999999999999999",
    ] {
        let toks = Lexer::new(src).tokenize();
        assert!(
            toks.iter().any(|t| matches!(t, Token::Invalid(_)))
                || matches!(toks[0], Token::IntLiteral(_) | Token::FloatLiteral(_)),
            "{src}: {toks:?}"
        );
    }
    // The ones that must be Invalid (not silently wrapped):
    for src in [
        "18446744073709551616",
        "340282366920938463463374607431768211455",
        "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        "0x",
        "9223372036854775808i64",
    ] {
        let toks = Lexer::new(src).tokenize();
        assert!(matches!(toks[0], Token::Invalid(_)), "{src}: {toks:?}");
    }
}

#[test]
fn u64_constants_reach_the_ast_with_their_type() {
    use windjammer::parser::{Expression, Item, Literal, Parser, Statement};
    let src = "fn main() {\n    let h = 14695981039346656037\n}\n";
    let toks = Lexer::new(src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(toks)));
    let program = parser.parse().expect("parses");
    let Item::Function { decl, .. } = &program.items[0] else { panic!() };
    let Statement::Let { value, .. } = decl.body[0] else { panic!("{:?}", decl.body) };
    let Expression::Literal { value: Literal::IntSuffixed(bits, suf), .. } = value else {
        panic!("{value:?}")
    };
    assert_eq!(suf, "u64");
    assert_eq!(*bits as u64, 14695981039346656037u64);
}
