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

//! Golden diagnostics: sources carry `//~ ERROR <substring>` markers on the line that must
//! report the error (rustc-ui style). Every marker must be matched by an error line reported at
//! exactly that line (with a column), and every reported error must be marked: no unmarked and
//! no missing diagnostics. Replaces hand-checking of diagnostics.

use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn errors(src: &str) -> Vec<(usize, usize, String)> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let Err(msg) = parser.parse() else { return vec![] };
    msg.lines()
        .map(|l| {
            let at = l.rfind("(at line ").unwrap_or_else(|| panic!("no position: {l}"));
            let tail = &l[at + "(at line ".len()..];
            let (line, rest) = tail.split_once(", column ").unwrap();
            let col: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            (line.parse().unwrap(), col.parse().unwrap(), l.to_string())
        })
        .collect()
}

fn check(name: &str, src: &str) {
    let got = errors(src);
    let mut expected: Vec<(usize, String)> = Vec::new();
    for (i, l) in src.lines().enumerate() {
        if let Some((_, m)) = l.split_once("//~ ERROR ") {
            expected.push((i + 1, m.trim().to_string()));
        }
    }
    for (line, needle) in &expected {
        assert!(
            got.iter().any(|(gl, gc, msg)| gl == line && *gc >= 1 && msg.contains(needle.as_str())),
            "{name}: expected error containing {needle:?} on line {line}; got {got:#?}"
        );
    }
    for (gl, _, msg) in &got {
        assert!(
            expected.iter().any(|(l, _)| l == gl),
            "{name}: unexpected error on line {gl} (no //~ ERROR marker): {msg}"
        );
    }
}

#[test]
fn attribute_hash_suggests_decorator() {
    check("attr", "#[derive(Clone)] //~ ERROR @derive(Clone)\nstruct S { x: int }\n");
}

#[test]
fn u128_literal_is_a_diagnostic() {
    check(
        "u128",
        "fn main() {\n    let a = 340282366920938463463374607431768211455 //~ ERROR does not fit in 64 bits\n}\n",
    );
}

#[test]
fn stray_characters() {
    check("tilde", "fn main() {\n    let x = 1 ~ 2 //~ ERROR unexpected character '~'\n}\n");
}

#[test]
fn several_errors_each_on_their_own_line() {
    check(
        "multi",
        "fn a() {\n    let = 1 //~ ERROR Expected pattern\n}\nfn b() {\n    let y = + //~ ERROR Unexpected token\n}\nstruct { } //~ ERROR Expected struct name\n",
    );
}

#[test]
fn malformed_range_pattern() {
    check(
        "range",
        "fn f(n: int) {\n    match n {\n        1..=x => 1, //~ ERROR range pattern bound\n    }\n}\n",
    );
}

#[test]
fn label_must_precede_a_loop() {
    check("label", "fn f() {\n    'a: let x = 1 //~ ERROR after label\n}\n");
}

#[test]
fn valid_programs_have_no_diagnostics() {
    check(
        "ok",
        "fn f(c: &mut int) {\n    'o: for i in 0..3 {\n        match i {\n            ..=0 => continue 'o,\n            1..=9 => break 'o,\n            _ => {}\n        }\n    }\n}\n",
    );
}
