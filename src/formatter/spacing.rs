//! Horizontal spacing between two adjacent tokens on one line.

use super::tokens::{Kind, Tok};

const PREFIX_KEYWORDS: &[&str] = &[
    "return", "in", "if", "else", "match", "while", "for", "let", "mut", "break", "move", "as",
    "yield", "await", "where", "loop", "unsafe", "defer", "thread", "async",
];

const SPACE_BEFORE_PAREN: &[&str] = &[
    "if", "else", "match", "while", "for", "in", "return", "loop", "let", "mut", "as", "move",
    "where", "unsafe", "async", "await", "break", "pub",
];

const MULTI_OPS: &[&str] = &[
    "<<=", ">>=", "..=", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "|>", "+=", "-=", "*=",
    "/=", "%=", "&=", "|=", "^=", "->", "<-", "=>", "::", "..", "//",
];

/// Tokens whose spacing is kept as written (angle brackets vs comparisons, closure pipes).
fn preserves(t: &Tok) -> bool {
    t.kind == Kind::Punct && matches!(t.text.as_str(), "<" | ">" | ">>" | "<<" | "|")
}

fn ends_operand(t: &Tok) -> bool {
    match t.kind {
        Kind::Word => !PREFIX_KEYWORDS.contains(&t.text.as_str()),
        Kind::Str | Kind::Char => true,
        Kind::Punct => matches!(t.text.as_str(), ")" | "]" | "}" | "?"),
        _ => false,
    }
}

/// Is `toks[i]` (one of `- * & && !`) in prefix position? `line_start` is the index of the
/// first token of its line.
pub fn is_unary(toks: &[Tok], i: usize, line_start: usize) -> bool {
    let t = &toks[i];
    if !(t.kind == Kind::Punct && matches!(t.text.as_str(), "-" | "*" | "&" | "&&" | "!")) {
        return false;
    }
    i == line_start || !ends_operand(&toks[i - 1])
}

/// `ident!` followed by a delimiter: a macro bang.
fn is_macro_bang(toks: &[Tok], i: usize, line_start: usize) -> bool {
    let t = &toks[i];
    t.is("!") && i > line_start && toks[i - 1].kind == Kind::Word && !t.space
        && toks.get(i + 1).is_some_and(|n| n.is_opener() && n.nl == 0)
}

fn is_prefix_op(toks: &[Tok], i: usize, ls: usize) -> bool {
    toks[i].kind == Kind::Punct && (is_macro_bang(toks, i, ls) || is_unary(toks, i, ls))
}

/// Should a space separate `toks[i-1]` and `toks[i]` (same line)?
pub fn space_between(toks: &[Tok], i: usize, ls: usize) -> bool {
    let (p, n) = (&toks[i - 1], &toks[i]);
    let want = decide(toks, i, ls, p, n);
    want || would_merge(p, n)
}

fn would_merge(p: &Tok, n: &Tok) -> bool {
    if p.kind != Kind::Punct || n.kind != Kind::Punct {
        return false;
    }
    let joined = format!("{}{}", p.text, n.text);
    MULTI_OPS
        .iter()
        .any(|op| joined.starts_with(op) && op.len() > p.text.len())
}

fn decide(toks: &[Tok], i: usize, ls: usize, p: &Tok, n: &Tok) -> bool {
    if n.is_comment() {
        return true;
    }
    if is_use_path_glue(toks, ls, p, n) {
        return n.space;
    }
    if let Some(v) = punctuation_rule(p, n) {
        return v;
    }
    if is_prefix_op(toks, i - 1, ls) {
        return false;
    }
    if is_macro_bang(toks, i, ls) {
        return false;
    }
    if let Some(v) = bracket_rule(toks, i, p, n) {
        return v;
    }
    if preserves(p) || preserves(n) {
        return n.space || (p.kind == Kind::Word && n.kind == Kind::Word);
    }
    if p.kind == Kind::Decorator && n.is("(") {
        return false;
    }
    true
}

/// Module path separators in `use` lines (`use ./a`, `use ../b`) keep their written spacing.
fn is_use_path_glue(toks: &[Tok], ls: usize, p: &Tok, n: &Tok) -> bool {
    let glue = |t: &Tok| t.kind == Kind::Punct && matches!(t.text.as_str(), "." | "/" | ".." | "*");
    toks[ls].kind == Kind::Word && toks[ls].text == "use" && (glue(p) || glue(n))
}

/// Commas, colons, dots and paths.
fn punctuation_rule(p: &Tok, n: &Tok) -> Option<bool> {
    let tight_before = matches!(n.text.as_str(), "," | ";" | "?" | "." | "::" | ":");
    if n.kind == Kind::Punct && tight_before {
        return Some(false);
    }
    if (n.is("..") || n.is("..=")) && ends_operand(p) {
        return Some(false);
    }
    if p.kind == Kind::Punct && matches!(p.text.as_str(), "." | "::" | ".." | "..=") {
        return Some(false);
    }
    if p.kind == Kind::Punct && matches!(p.text.as_str(), "," | ";" | ":") {
        return Some(true);
    }
    None
}

fn bracket_rule(toks: &[Tok], i: usize, p: &Tok, n: &Tok) -> Option<bool> {
    if p.is("{") {
        return Some(!n.is("}"));
    }
    if n.is("}") {
        return Some(true);
    }
    if p.is("(") || p.is("[") || n.is(")") || n.is("]") {
        return Some(false);
    }
    if n.is("{") {
        return Some(!toks[i - 1].is("!"));
    }
    if n.is("(") && i >= 2 && toks[i - 2].kind == Kind::Word && toks[i - 2].text == "fn" {
        return Some(false);
    }
    if n.is("(") || n.is("[") {
        return Some(open_needs_space(p, n));
    }
    None
}

fn open_needs_space(p: &Tok, n: &Tok) -> bool {
    match p.kind {
        Kind::Word => SPACE_BEFORE_PAREN.contains(&p.text.as_str()),
        Kind::Decorator => false,
        Kind::Punct => match p.text.as_str() {
            ")" | "]" | "?" | "!" => false,
            "<" | ">" | ">>" | "|" => n.space,
            "}" => true,
            _ => true,
        },
        _ => true,
    }
}
