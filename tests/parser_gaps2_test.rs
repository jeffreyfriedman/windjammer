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

//! Parser gaps, round 2: STMT-011 (bare `return` + newline), GRAM-004 (`[` starts a statement),
//! TRT-008 (bounds with type arguments), TRT-007 (generic trait methods), Q-004 (`-1 as u8`),
//! Q-002 (nested block comments), Q-010 (generic type aliases). Backend behaviour:
//! tests/conformance_programs/160..163.

use windjammer::lexer::{Lexer, Token};
use windjammer::parser::{Item, Parser, Program};

fn parse(src: &str) -> Result<Program<'static>, String> {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse()
}

fn dbg(src: &str) -> String {
    format!("{:?}", parse(src).unwrap_or_else(|e| panic!("did not parse: {e}\n{src}")).items)
}

#[test]
fn bare_return_does_not_swallow_the_next_line() {
    let d = dbg("fn f() {\n    return\n    g()\n}\n");
    assert!(d.contains("Return { value: None"), "{d}");
    assert!(d.contains("Expression { expr: Call"), "next line must be its own statement: {d}");
    // value on the same line is still a value
    let d = dbg("fn f() -> int {\n    return g()\n}\n");
    assert!(d.contains("Return { value: Some(Call"), "{d}");
    // match arm `=> return` followed by a newline
    let d = dbg("fn f(x: int) {\n    match x {\n        0 => return\n        ,\n        _ => {}\n    }\n}\n");
    assert!(d.contains("Return { value: None"), "{d}");
}

#[test]
fn bracket_at_line_start_begins_a_statement() {
    let d = dbg("fn f() {\n    let a = g()\n    [1, 2].len()\n}\n");
    assert!(!d.contains("Index {"), "`[` on a new line must not index the previous line: {d}");
    assert!(d.contains("Array {"), "{d}");
    // same-line indexing still works
    assert!(dbg("fn f() {\n    let a = xs[0]\n}\n").contains("Index {"));
}

#[test]
fn trait_bounds_take_type_arguments() {
    let d = dbg("fn f<T: Into<string> + Clone, U: Iterator<Item = int>>(t: T) {}\n".replace("Item = int", "int").as_str());
    assert!(d.contains("\"Into<string>\""), "{d}");
    assert!(d.contains("\"Clone\""), "{d}");
    let d = dbg("fn g<T>(t: T) where T: AsRef<Vec<int>> {}\n");
    assert!(d.contains("AsRef<Vec<int>>"), "{d}");
}

#[test]
fn generic_methods_inside_traits() {
    let p = parse("trait Conv {\n    fn conv<T: Clone>(self, t: T) -> T\n    fn plain(self) -> int\n}\n").unwrap();
    let Item::Trait { decl, .. } = &p.items[0] else { panic!() };
    assert_eq!(decl.methods[0].type_params.len(), 1);
    assert_eq!(decl.methods[0].type_params[0].bounds, vec!["Clone".to_string()]);
    assert!(decl.methods[1].type_params.is_empty());
}

#[test]
fn unary_minus_binds_tighter_than_as() {
    let d = dbg("fn f() {\n    let a = -1 as u8\n}\n");
    assert!(d.contains("Cast { expr: Unary { op: Neg"), "`-1 as u8` must be `(-1) as u8`: {d}");
    // postfix calls still bind tighter than prefix minus: -x.abs() == -(x.abs())
    let d = dbg("fn f(x: int) {\n    let a = -x.abs()\n}\n");
    assert!(d.contains("Unary { op: Neg, operand: MethodCall"), "{d}");
    // `as` inside parentheses under a prefix operator stays inside
    let d = dbg("fn f() {\n    let a = -(1 as i64)\n}\n");
    assert!(d.contains("Unary { op: Neg, operand: Cast"), "{d}");
    for op in ["!", "*"] {
        let d = dbg(&format!("fn f(x: bool) {{\n    let a = {op}x as u8\n}}\n"));
        assert!(d.contains("Cast { expr: Unary"), "{op}: {d}");
    }
}

#[test]
fn block_comments_nest_and_are_whitespace() {
    let toks = Lexer::new("a /* x /* y */ z */ b").tokenize();
    assert_eq!(toks, vec![Token::Ident("a".into()), Token::Ident("b".into()), Token::Eof]);
    let toks = Lexer::new("a /* never closed /* */").tokenize();
    assert!(toks.iter().any(|t| matches!(t, Token::Invalid(_))), "{toks:?}");
    // a comment that spans lines still separates statements
    let d = dbg("fn f() {\n    let a = g() /* c\n */\n    [1].len()\n}\n");
    assert!(!d.contains("Index {"), "{d}");
    // positions after a multi-line comment stay right
    let t = Lexer::new("/* a\nb */ x").tokenize_with_locations();
    assert_eq!((t[0].line, t[0].column), (2, 6), "{t:?}");
}

#[test]
fn generic_type_aliases() {
    let p = parse("type Pair<T> = (T, T)\ntype Plain = int\n").unwrap();
    let Item::TypeAlias { type_params, .. } = &p.items[0] else { panic!() };
    assert_eq!(type_params.len(), 1);
    let Item::TypeAlias { type_params, .. } = &p.items[1] else { panic!() };
    assert!(type_params.is_empty());
}
