//! Gate 2: AST location invariants (corpus + hand-written + mutation self-tests).

use crate::span_oracle::ast_oracle::*;
use crate::span_oracle::corpus::*;
use crate::span_oracle::ratchet;
use std::collections::BTreeMap;
use windjammer::parser::ast::*;
use windjammer::parser::NodeSpan;
use std::collections::HashMap;

fn findings(src: &str) -> Findings {
    match check_source("t.wj", src) {
        ParseOutcome::Ok(f) => f,
        ParseOutcome::ParseError(e) => panic!("parse error: {e}\n{src}"),
        ParseOutcome::LexerPanic(e) => panic!("lexer panic: {e}\n{src}"),
    }
}

fn classes(f: &Findings) -> Vec<String> {
    f.by_class.iter().map(|(k, (n, ex))| format!("{k} x{n}: {ex}")).collect()
}

const SNIPPETS: &[&str] = &[
    "fn main() {\n    let x = 1\n    let y = x\n    y\n}\n",
    "fn f(a: int, b: int) -> int {\n    a + b * 2\n}\n",
    "fn g() {\n    let s = \"a ${x} b ${y + 1}\"\n    println(s)\n}\n",
    "fn g() {\r\n\tlet é = 1\r\n\tlet z = é + 1\r\n}\r\n",
    "fn h(v: Vec<int>) {\n    for i in v {\n        if i > 1 {\n            return\n        }\n    }\n}\n",
    "fn k() {\n    let r = foo.bar(1, 2).baz\n    let c = |a| a + 1\n}\n",
    "fn m(x: int) {\n    match x {\n        1 => 2,\n        _ => 3,\n    }\n}\n",
];

#[test]
fn hand_written_programs_have_sound_locations() {
    for src in SNIPPETS {
        let f = findings(src);
        assert!(f.by_class.is_empty(), "{src:?}\n{}", classes(&f).join("\n"));
    }
}

#[test]
fn repo_corpus_location_classes_are_ratcheted() {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut parse_fail = std::collections::BTreeSet::new();
    let mut examples: BTreeMap<String, String> = BTreeMap::new();
    for p in repo_wj_files() {
        let Ok(src) = std::fs::read_to_string(&p) else { continue };
        match check_source(&rel(&p), &src) {
            ParseOutcome::Ok(f) => {
                for (k, (n, ex)) in f.by_class {
                    *counts.entry(k.clone()).or_default() += n;
                    examples.entry(k).or_insert_with(|| format!("{}: {ex}", rel(&p)));
                }
            }
            ParseOutcome::ParseError(e) | ParseOutcome::LexerPanic(e) => {
                // Negative programs written for the diagnostics gate fail to parse on purpose.
                if !rel(&p).starts_with("tests/diagnostics_oracle/") {
                    if std::env::var_os("SPAN_ORACLE_VERBOSE").is_some() {
                        eprintln!("PARSEFAIL {}: {}", rel(&p), e.lines().next().unwrap_or(""));
                    }
                    parse_fail.insert(rel(&p));
                }
            }
        }
    }
    if std::env::var_os("SPAN_ORACLE_VERBOSE").is_some() {
        for (k, e) in &examples {
            eprintln!("EXAMPLE {k}: {e}");
        }
    }
    ratchet::enforce_counts("ast_location_classes", &counts);
    ratchet::enforce("ast_parse_failures", &parse_fail);
}

// ---- mutation self-tests: hand-built ASTs + span tables must trip each rule -------------------

const SRC: &str = "fn f() {\n    ab + cd\n}\n";

struct Table(HashMap<usize, NodeSpan>);
impl SpanSource for Table {
    fn expr(&self, e: &Expression) -> Option<NodeSpan> {
        self.0.get(&(e as *const Expression as usize)).copied()
    }
    fn stmt(&self, s: &Statement) -> Option<NodeSpan> {
        self.0.get(&(s as *const Statement as usize)).copied()
    }
}

/// Span covering `text` (n-th occurrence) in SRC, computed by the naive oracle.
fn span_of(text: &str, nth: usize) -> NodeSpan {
    let start = SRC.match_indices(text).nth(nth).expect("text in SRC").0;
    span_bytes(start, start + text.len())
}
fn span_bytes(bs: usize, be: usize) -> NodeSpan {
    let (line, column) = crate::span_oracle::naive::line_col(SRC, bs);
    let (end_line, end_column) = crate::span_oracle::naive::line_col(SRC, be.min(SRC.len()));
    NodeSpan { byte_start: bs, byte_end: be, line, column, end_line, end_column }
}

struct Built {
    table: HashMap<usize, NodeSpan>,
}
impl Built {
    fn new() -> Self {
        Built { table: HashMap::new() }
    }
    fn e(&mut self, e: Expression<'static>, span: Option<NodeSpan>) -> &'static Expression<'static> {
        let r: &'static Expression<'static> = Box::leak(Box::new(e));
        if let Some(s) = span {
            self.table.insert(r as *const Expression as usize, s);
        }
        r
    }
    fn ident(&mut self, name: &str, span: Option<NodeSpan>) -> &'static Expression<'static> {
        self.e(Expression::Identifier { name: name.into(), location: None }, span)
    }
    /// Check `e` as the only statement of a function whose statement spans `stmt_span`.
    fn run(mut self, e: &'static Expression<'static>, stmt_span: NodeSpan) -> Findings {
        let stmt: &'static Statement<'static> =
            Box::leak(Box::new(Statement::Expression { expr: e, location: None }));
        self.table.insert(stmt as *const Statement as usize, stmt_span);
        let item = Item::Function {
            decl: FunctionDecl {
                name: "f".into(),
                is_pub: false,
                is_extern: false,
                type_params: vec![],
                where_clause: vec![],
                decorators: vec![],
                is_async: false,
                parameters: vec![],
                return_type: None,
                return_decorators: vec![],
                body: vec![stmt],
                parent_type: None,
                impl_trait: None,
                doc_comment: None,
            },
            location: None,
        };
        check_items(SRC, &[item], &Table(self.table))
    }
}

fn binary(b: &mut Built, whole: Option<NodeSpan>, ab: Option<NodeSpan>, cd: Option<NodeSpan>) -> &'static Expression<'static> {
    let l = b.ident("ab", ab);
    let r = b.ident("cd", cd);
    b.e(Expression::Binary { left: l, op: BinaryOp::Add, right: r, location: None }, whole)
}

fn good() -> (Built, &'static Expression<'static>) {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(span_of("ab", 0)), Some(span_of("cd", 0)));
    (b, e)
}

fn rejected(rule: &str, f: Findings) {
    assert!(f.by_class.keys().any(|k| k.starts_with(rule)), "rule `{rule}` not triggered; got {:?}", f.by_class.keys().collect::<Vec<_>>());
}

#[test]
fn oracle_accepts_correct_hand_built_ast() {
    let (b, e) = good();
    let f = b.run(e, span_of("ab + cd", 0));
    assert!(f.by_class.is_empty(), "{:?}", classes(&f));
}
#[test]
fn oracle_rejects_missing_span() {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), None, Some(span_of("cd", 0)));
    rejected("no-span", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_range_outside_source() {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(span_of("ab", 0)), Some(span_bytes(SRC.len() - 1, SRC.len() + 5)));
    rejected("bad-range", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_start_after_end() {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(span_bytes(14, 13)), Some(span_of("cd", 0)));
    rejected("bad-range", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_empty_span() {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(span_bytes(13, 13)), Some(span_of("cd", 0)));
    rejected("empty-span", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_column_inconsistent_with_byte_offset() {
    let mut b = Built::new();
    let mut s = span_of("ab", 0);
    s.column += 1;
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(s), Some(span_of("cd", 0)));
    rejected("line-col-mismatch", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_end_line_inconsistent_with_byte_offset() {
    let mut b = Built::new();
    let mut s = span_of("cd", 0);
    s.end_line += 1;
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(span_of("ab", 0)), Some(s));
    rejected("line-col-mismatch", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_child_outside_parent() {
    let mut b = Built::new();
    // parent covers only `ab +`, right child `cd` lies outside it
    let e = binary(&mut b, Some(span_bytes(13, 17)), Some(span_of("ab", 0)), Some(span_of("cd", 0)));
    rejected("outside-parent", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_node_located_at_following_token() {
    // The historical class: an identifier whose span is the token AFTER it.
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab + cd", 0)), Some(span_of("+", 0)), Some(span_of("cd", 0)));
    rejected("wrong-first-token", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_binary_not_ending_at_right_operand() {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("ab +", 0)), Some(span_of("ab", 0)), Some(span_of("cd", 0)));
    rejected("end-not-last-child", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_binary_not_starting_at_left_operand() {
    let mut b = Built::new();
    let e = binary(&mut b, Some(span_of("+ cd", 0)), Some(span_of("ab", 0)), Some(span_of("cd", 0)));
    rejected("start-not-first-child", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_siblings_out_of_order() {
    let mut b = Built::new();
    let cd = b.ident("cd", Some(span_of("cd", 0)));
    let ab = b.ident("ab", Some(span_of("ab", 0)));
    let e = b.e(Expression::Array { elements: vec![cd, ab], location: None }, Some(span_of("ab + cd", 0)));
    rejected("siblings-out-of-order", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_slice_that_reparses_to_another_kind() {
    // An identifier node whose span covers `ab + cd` (starts with its name, but is a Binary).
    let mut b = Built::new();
    let e = b.ident("ab", Some(span_of("ab + cd", 0)));
    rejected("reparse-kind-mismatch", b.run(e, span_of("ab + cd", 0)));
}
#[test]
fn oracle_rejects_wrong_last_token() {
    let mut b = Built::new();
    let f = b.ident("f", Some(span_of("ab", 0)));
    let e = b.e(Expression::Call { function: f, arguments: vec![], location: None }, Some(span_of("ab", 0)));
    rejected("wrong-last-token", b.run(e, span_of("ab + cd", 0)));
}
