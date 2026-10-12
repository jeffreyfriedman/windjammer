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

//! Parser grammar gaps (range patterns, labeled loops, string-first interpolation, statement
//! boundaries, turbofish scrutinee, `self::f` vs `self.f`, error positions).
//! Behaviour on every backend is checked by tests/conformance_programs/15x_*.wj.

use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Program};

pub fn parse(src: &str) -> Result<Program<'static>, String> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    // The AST borrows from the parser's arenas: keep the parser alive (as the CLI does).
    let parser = Box::leak(Box::new(Parser::new_with_source(tokens, "t.wj".to_string(), src.to_string())));
    parser.parse()
}

/// Debug rendering of the AST with every `location: ...` removed (spans differ between layouts).
pub fn fingerprint(src: &str) -> Result<String, String> {
    let program = parse(src)?;
    let dbg = format!("{:?}", program.items);
    Ok(strip_locations(&dbg))
}

pub fn strip_locations(dbg: &str) -> String {
    let b = dbg.as_bytes();
    let mut out = String::with_capacity(dbg.len());
    let mut i = 0;
    while i < b.len() {
        if dbg[i..].starts_with("location: ") {
            i += "location: ".len();
            if dbg[i..].starts_with("None") {
                i += 4;
            } else if dbg[i..].starts_with("Some(") {
                let mut depth = 0;
                while i < b.len() {
                    match b[i] {
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            // drop a following ", " so `a, location: x, b` -> `a, b`
            if dbg[i..].starts_with(", ") {
                i += 2;
            }
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

fn body_dbg(src: &str) -> String {
    fingerprint(src).unwrap_or_else(|e| panic!("did not parse: {e}\n{src}"))
}

// ---- (1) range patterns -----------------------------------------------------------------

#[test]
fn range_patterns_parse_to_range_nodes() {
    for (pat, want) in [
        ("1..=9", "Range { start: Some(Int(1)), end: Some(Int(9)), inclusive: true }"),
        ("1..9", "Range { start: Some(Int(1)), end: Some(Int(9)), inclusive: false }"),
        ("..=9", "Range { start: None, end: Some(Int(9)), inclusive: true }"),
        ("..9", "Range { start: None, end: Some(Int(9)), inclusive: false }"),
        ("100..", "Range { start: Some(Int(100)), end: None, inclusive: false }"),
        ("-5..=-1", "Range { start: Some(Int(-5)), end: Some(Int(-1)), inclusive: true }"),
        ("'a'..='z'", "Range { start: Some(Char('a')), end: Some(Char('z')), inclusive: true }"),
        ("0.5..1.5", "Range { start: Some(Float(0.5)), end: Some(Float(1.5)), inclusive: false }"),
    ] {
        let src = format!("fn f(n: int) -> int {{\n    match n {{\n        {pat} => 1,\n        _ => 0,\n    }}\n}}\n");
        let d = body_dbg(&src);
        assert!(d.contains(want), "{pat}: expected `{want}` in\n{d}");
    }
}

#[test]
fn range_patterns_combine_with_or_guards_and_nesting() {
    for pat in ["1..=3 | 7..=9", "Some(1..=9)", "(0..5, 'a'..='f')", "x if x > 3"] {
        let src = format!("fn f(n: int) {{\n    match n {{\n        {pat} => println(\"a\"),\n        _ => println(\"b\"),\n    }}\n}}\n");
        body_dbg(&src);
    }
    // rest patterns are not confused with ranges
    body_dbg("struct P { a: int, b: int }\nfn f(p: P) {\n    match p {\n        P { a, .. } => println(\"a\"),\n    }\n}\n");
}

#[test]
fn malformed_range_pattern_is_an_error_with_position() {
    let e = fingerprint("fn f(n: int) {\n    match n {\n        1..=x => 1,\n    }\n}\n").unwrap_err();
    assert!(e.contains("range pattern bound") && e.contains("line 3"), "{e}");
}

// ---- (2) labeled loops ------------------------------------------------------------------

#[test]
fn labeled_loops_break_continue_and_values() {
    let d = body_dbg(
        "fn f() {\n    'outer: for i in 0..3 {\n        'inner: while true {\n            continue 'outer\n        }\n        break 'outer\n    }\n    let v = 'l: loop {\n        break 'l 5\n    }\n    let w = loop {\n        break 7\n    }\n}\n",
    );
    for want in [
        "label: Some(\"outer\")",
        "label: Some(\"inner\")",
        "Continue { label: Some(\"outer\")",
        "Break { label: Some(\"outer\"), value: None",
        "Break { label: Some(\"l\"), value: Some(Literal { value: Int(5)",
        "Break { label: None, value: Some(Literal { value: Int(7)",
    ] {
        assert!(d.contains(want), "missing `{want}` in\n{d}");
    }
}

#[test]
fn break_value_only_on_same_line() {
    let d = body_dbg("fn f() {\n    loop {\n        break\n        foo()\n    }\n}\n");
    assert!(d.contains("Break { label: None, value: None"), "{d}");
}

#[test]
fn label_without_loop_is_an_error() {
    let e = fingerprint("fn f() {\n    'a: let x = 1\n}\n").unwrap_err();
    assert!(e.contains("after label") && e.contains("line 2"), "{e}");
}

// ---- (5) interpolation starting with a string literal ------------------------------------

#[test]
fn string_literal_first_in_interpolation() {
    let d = body_dbg("fn f() {\n    let a = \"${\"abc\".len()}\"\n    let b = \"${\"\n}\n");
    assert!(d.contains("len"), "{d}");
    // `${"` whose nested string is not followed by `.`/`[` stays a literal dollar-brace
    // (so `"${"` is the two characters `${`); documented in docs/design/lexical-grammar.md.
    assert!(d.contains("String(\"${\")"), "{d}");
}

// ---- (6) statement boundaries ------------------------------------------------------------

#[test]
fn star_line_after_call_is_a_deref_assignment() {
    let d = body_dbg("fn f(c: &mut int) {\n    g(1)\n    *c = 5\n}\n");
    assert!(d.contains("Assignment") && d.contains("op: Deref"), "{d}");
    assert!(!d.contains("Mul"), "multiplication continuation leaked: {d}");
}

#[test]
fn star_continuation_without_assignment_stays_multiplication() {
    let d = body_dbg("fn f(w: int, h: int) -> int {\n    let a = w\n        * h\n    a\n}\n");
    assert!(d.contains("Mul"), "{d}");
}

/// Property: for random statement sequences, the newline-separated program and the `;`-separated
/// one-line program have identical ASTs (the boundary rule is the only thing that differs).
#[test]
fn split_line_equals_one_line_property() {
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }
    let pool = [
        "g(1)",
        "g(a, b)",
        "*c = 1",
        "*c += g(2)",
        "*c *= *c",
        "*c = *c * 2",
        "let v = g(3)",
        "let w = a * b",
        "a = a * 3",
        "h.push(g(1))",
        "(*c) = 4",
        "x.y = g(1)",
        "*p.q = 2",
        "let z = (a + b) * 2",
        "if a > 1 { *c = 3 }",
        "g(1) * 2",
        "[1, 2].len()",
        "[a, b]",
        "xs[0] = 1",
    ];
    let mut rng = Rng(0xABCDEF0123456789);
    for case in 0..400 {
        let n = 2 + (rng.next() % 6) as usize;
        let stmts: Vec<&str> = (0..n).map(|_| pool[(rng.next() % pool.len() as u64) as usize]).collect();
        let split = format!("fn t() {{\n    {}\n}}\n", stmts.join("\n    "));
        let one = format!("fn t() {{ {} }}\n", stmts.join("; "));
        let a = fingerprint(&split);
        let b = fingerprint(&one);
        assert_eq!(a, b, "case {case}: split vs one-line differ for\n{split}");
    }
}

// ---- (8) turbofish in match scrutinee ----------------------------------------------------

#[test]
fn turbofish_method_call_in_match_scrutinee() {
    let d = body_dbg("fn f(s: string) {\n    match s.parse::<int>() {\n        Ok(v) => println(\"a\"),\n        Err(_) => println(\"b\"),\n    }\n}\n");
    assert!(d.contains("MethodCall") && d.contains("method: \"parse\""), "{d}");
    assert!(d.contains("type_args: Some("), "{d}");
    // also in `if` conditions and `while` conditions (same scrutinee parser)
    body_dbg("fn f(s: string) {\n    if s.parse::<int>().is_ok() {\n        println(\"a\")\n    }\n}\n");
}

// ---- (9) self::f vs self.f ----------------------------------------------------------------

#[test]
fn self_path_call_and_self_method_call_are_distinct_asts() {
    let path = body_dbg("impl S {\n    fn m(self) -> int {\n        self::f()\n    }\n}\n");
    let method = body_dbg("impl S {\n    fn m(self) -> int {\n        self.f()\n    }\n}\n");
    assert_ne!(path, method, "self::f() and self.f() parsed to the same AST");
    assert!(path.contains("Call {") && path.contains("name: \"self::f\""), "{path}");
    assert!(method.contains("MethodCall") && method.contains("method: \"f\""), "{method}");
}

// ---- (3) attributes -------------------------------------------------------------------------

#[test]
fn hash_attribute_suggests_decorator() {
    let e = fingerprint("#[derive(Clone)]\nstruct S { x: int }\n").unwrap_err();
    assert!(e.contains("@derive(Clone)") && e.contains("line 1, column 1"), "{e}");
    let e = fingerprint("#[test]\nfn t() {}\n").unwrap_err();
    assert!(e.contains("@test"), "{e}");
}

#[test]
fn decorators_still_parse() {
    body_dbg("@derive(Clone, Debug)\nstruct S { x: int }\n@test\nfn t() {}\n");
}

#[test]
fn continue_does_not_take_a_label_from_the_next_line() {
    let d = body_dbg("fn f() {\n    loop {\n        continue\n        'outer: loop {\n            break 'outer\n        }\n    }\n}\n");
    assert!(d.contains("Continue { label: None"), "{d}");
    assert!(d.contains("label: Some(\"outer\")"), "{d}");
}
